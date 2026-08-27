// Команда oh-probe — консольный бот-клиент: подключается, здоровается, ходит
// по кругу и печатает, что видит. Нужен для смоук-проверки сервера без игры
// и для нагрузочных прогонов (этап N6).
//
//	oh-probe -addr ws://127.0.0.1:7777/ws -nick bot1 -seconds 5
package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"math"
	"os"
	"time"

	"github.com/villetacore/open-heart/server/internal/proto"
	"github.com/villetacore/open-heart/server/internal/wire"
)

func main() {
	addr := flag.String("addr", "ws://127.0.0.1:7777/ws", "адрес сервера")
	nick := flag.String("nick", "probe", "ник")
	preset := flag.String("preset", "", "пресет (пусто = не проверять)")
	hash := flag.String("hash", "", "content_hash (пусто = не проверять)")
	seconds := flag.Int("seconds", 5, "сколько секунд ходить")
	quiet := flag.Bool("quiet", false, "не печатать снапшоты")
	attack := flag.Bool("attack", false, "стрелять по ближайшему врагу")
	delve := flag.Int("delve", 0, "уйти в забег указанной глубины после входа")
	join := flag.Int("join", 0, "peer ведущего забега: зайти в его инстанс, а не в свой")
	flag.Parse()

	if err := probe(*addr, *nick, *preset, *hash, *seconds, *quiet, *attack, *delve, *join); err != nil {
		fmt.Fprintln(os.Stderr, "oh-probe:", err)
		os.Exit(1)
	}
}

// walkTarget — куда бот хочет шагнуть в этом тике.
//
// Шаг ограничен: сервер проверяет правдоподобие скорости и отбросит рывок.
func walkTarget(pos, base proto.Vec3, snap proto.Snapshot, attack bool, phase float64) proto.Vec3 {
	const step = 0.25

	// В боевом режиме идём к ближайшему врагу — так бот попадает в драку.
	if attack && len(snap.Enemies) > 0 {
		best, bestDist := snap.Enemies[0], math.MaxFloat64
		for _, enemy := range snap.Enemies {
			if d := dist(pos, enemy.Pos); d < bestDist {
				best, bestDist = enemy, d
			}
		}
		if bestDist > 1.5 {
			dx := float64(best.Pos[0] - pos[0])
			dz := float64(best.Pos[2] - pos[2])
			length := math.Sqrt(dx*dx + dz*dz)
			if length > 0.01 {
				return proto.Vec3{
					pos[0] + float32(dx/length*step),
					pos[1],
					pos[2] + float32(dz/length*step),
				}
			}
		}
		return pos
	}

	// Иначе нарезаем круги вокруг точки появления.
	const radius = 3.0
	return proto.Vec3{
		base[0] + float32(radius*math.Cos(phase)),
		pos[1],
		base[2] + float32(radius*math.Sin(phase)),
	}
}

func dist(a, b proto.Vec3) float64 {
	dx := float64(a[0] - b[0])
	dy := float64(a[1] - b[1])
	dz := float64(a[2] - b[2])
	return math.Sqrt(dx*dx + dy*dy + dz*dz)
}

func probe(addr, nick, preset, hash string, seconds int, quiet, attack bool, delve, join int) error {
	ctx, cancel := context.WithTimeout(context.Background(),
		time.Duration(seconds+10)*time.Second)
	defer cancel()

	conn, err := wire.Dial(ctx, addr)
	if err != nil {
		return err
	}
	defer conn.Close("bye")

	err = conn.Send(proto.THello, proto.Hello{
		Protocol:    proto.Version,
		GameVersion: "probe",
		PresetID:    preset,
		ContentHash: hash,
		Auth:        proto.Auth{Mode: "open", Nickname: nick},
	})
	if err != nil {
		return err
	}

	var peer uint16
	var myPos, base proto.Vec3
	var tick uint32
	snaps, events := 0, 0
	hits, damage := 0, 0.0
	deadline := time.After(time.Duration(seconds) * time.Second)

	for {
		select {
		case <-deadline:
			fmt.Printf("[%s] итог: peer=%d снапшотов=%d событий=%d заявок=%d урона=%.0f\n",
				nick, peer, snaps, events, hits, damage)
			return nil
		default:
		}

		readCtx, cancelRead := context.WithTimeout(ctx, 5*time.Second)
		env, err := conn.Read(readCtx)
		cancelRead()
		if err != nil {
			return err
		}

		switch env.T {
		case proto.TWelcome:
			var wmsg proto.Welcome
			if err := env.Into(&wmsg); err != nil {
				return err
			}
			peer = wmsg.Peer
			fmt.Printf("[%s] welcome: peer=%d пресет=%s мир=%s сид=%d игроков=%d\n",
				nick, wmsg.Peer, wmsg.PresetID, wmsg.World.Kind, wmsg.World.Seed,
				len(wmsg.Players))
			base = proto.Vec3{}
			// Уходим в забег: сервер переведёт нас в отдельную комнату
			// и пришлёт новый welcome уже оттуда.
			if delve > 0 && wmsg.World.Kind == "hub" {
				args, _ := json.Marshal(map[string]int{"depth": delve, "owner": join})
				_ = conn.Send(proto.TCmd, proto.Cmd{Kind: "enter_delve", Args: args})
			}
		case proto.TSnapshot:
			var snap proto.Snapshot
			if err := env.Into(&snap); err != nil {
				return err
			}
			snaps++
			for _, ent := range snap.Players {
				if ent.ID == peer {
					myPos = ent.Pos
					if base == (proto.Vec3{}) {
						base = ent.Pos
					}
				}
			}
			// Стрельба: выбираем ближайшего врага и заявляем попадание.
			// Сервер сам решит, засчитывать его или нет.
			if attack && len(snap.Enemies) > 0 {
				best, bestDist := snap.Enemies[0], math.MaxFloat64
				for _, enemy := range snap.Enemies {
					if d := dist(myPos, enemy.Pos); d < bestDist {
						best, bestDist = enemy, d
					}
				}
				if bestDist < 40 {
					_ = conn.Send(proto.TFire, proto.Fire{
						Tick: snap.Tick, Weapon: "pistol", Origin: myPos,
					})
					_ = conn.Send(proto.THitClaim, proto.HitClaim{
						Tick: snap.Tick, Weapon: "pistol", Target: best.ID, Pos: best.Pos,
					})
					hits++
				}
			}
			// Шаг: сервер принимает только достижимые перемещения, поэтому
			// идём от собственной позиции, а не от нуля.
			tick++
			step := walkTarget(myPos, base, snap, attack, float64(tick)/20.0)
			_ = conn.Send(proto.TInput, proto.Input{
				Tick: tick, Pos: step, OnFloor: true,
				Yaw: float32(math.Atan2(float64(step[0]-myPos[0]), float64(step[2]-myPos[2]))),
			})

			if !quiet && snaps%20 == 0 {
				fmt.Printf("[%s] snap #%d: tick=%d ack=%d игроков=%d врагов=%d\n",
					nick, snaps, snap.Tick, snap.Ack, len(snap.Players), len(snap.Enemies))
			}
		case proto.TEvent:
			var ev proto.Event
			if err := env.Into(&ev); err != nil {
				return err
			}
			events++
			if ev.Kind == proto.EvDamage {
				// Урона за забег много — печатаем его одним числом в итоге.
				damage += float64(ev.Amount)
				continue
			}
			fmt.Printf("[%s] событие %s actor=%d target=%d %s\n",
				nick, ev.Kind, ev.Actor, ev.Target, ev.Text)
		case proto.TReject:
			var rej proto.Reject
			if err := env.Into(&rej); err != nil {
				return err
			}
			return fmt.Errorf("отказ %s: %s", rej.Reason, rej.Detail)
		}
	}
}
