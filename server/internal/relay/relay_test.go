package relay

import (
	"bytes"
	"context"
	"errors"
	"io"
	"net"
	"testing"
	"time"

	"github.com/hashicorp/yamux"

	"github.com/villetacore/open-heart/server/internal/proto"
)

func TestFrameRoundTrip(t *testing.T) {
	var buf bytes.Buffer
	messages := [][]byte{[]byte(`{"t":"ping"}`), {}, bytes.Repeat([]byte("x"), MaxFrame)}
	for _, m := range messages {
		if err := WriteFrame(&buf, m); err != nil {
			t.Fatalf("запись: %v", err)
		}
	}
	for i, want := range messages {
		got, err := ReadFrame(&buf)
		if err != nil {
			t.Fatalf("чтение %d: %v", i, err)
		}
		if !bytes.Equal(got, want) {
			t.Fatalf("кадр %d: получено %d байт, ожидалось %d", i, len(got), len(want))
		}
	}
	if _, err := ReadFrame(&buf); !errors.Is(err, io.EOF) {
		t.Fatalf("после последнего кадра ожидался EOF, получено %v", err)
	}
}

// Кадр больше лимита не должен ни уходить, ни приниматься: иначе туннель
// пропустит то, что прямое соединение отвергло бы.
func TestFrameTooBig(t *testing.T) {
	var buf bytes.Buffer
	if err := WriteFrame(&buf, bytes.Repeat([]byte("x"), MaxFrame+1)); !errors.Is(err, ErrFrameTooBig) {
		t.Fatalf("запись: ожидался ErrFrameTooBig, получено %v", err)
	}
	if buf.Len() != 0 {
		t.Fatalf("в поток ушло %d байт при отказе", buf.Len())
	}

	header := []byte{0x00, 0x02, 0x00, 0x00} // заявлено 128 КиБ
	if _, err := ReadFrame(bytes.NewReader(header)); !errors.Is(err, ErrFrameTooBig) {
		t.Fatalf("чтение: ожидался ErrFrameTooBig, получено %v", err)
	}
}

// StreamConn снаружи должен вести себя как обычное соединение: отправленное
// уходит кадрами, принятое разбирается как конверт протокола.
func TestStreamConnSendAndRead(t *testing.T) {
	ours, theirs := net.Pipe()
	conn := NewStreamConn(ours, "1.2.3.4:5000")
	defer conn.Close("тест")

	if conn.Addr() != "1.2.3.4:5000" {
		t.Fatalf("адрес: %q", conn.Addr())
	}

	if err := conn.Send(proto.TReject, proto.Reject{Reason: "full", Detail: "мест нет"}); err != nil {
		t.Fatalf("отправка: %v", err)
	}
	payload, err := ReadFrame(theirs)
	if err != nil {
		t.Fatalf("другая сторона не получила кадр: %v", err)
	}
	if !bytes.Contains(payload, []byte(`"full"`)) {
		t.Fatalf("в кадре нет причины отказа: %s", payload)
	}

	go func() { _ = WriteFrame(theirs, []byte(`{"t":"ping","d":{}}`)) }()
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	env, err := conn.Read(ctx)
	if err != nil {
		t.Fatalf("чтение: %v", err)
	}
	if env.T != "ping" {
		t.Fatalf("тип конверта: %q", env.T)
	}
}

// Закрытие должно быть идемпотентным и разблокировать всех ждущих Done.
func TestStreamConnClose(t *testing.T) {
	ours, _ := net.Pipe()
	conn := NewStreamConn(ours, "relay")
	conn.Close("раз")
	conn.Close("два")

	select {
	case <-conn.Done():
	case <-time.After(time.Second):
		t.Fatal("Done не закрылся")
	}
	if err := conn.SendRaw([]byte("поздно")); !errors.Is(err, net.ErrClosed) {
		t.Fatalf("отправка в закрытое: %v", err)
	}
}

// Хаб адресует туннели и по id сервера, и по токену входа, а повторное
// подключение вытесняет прежний туннель.
func TestHubLookupAndReplace(t *testing.T) {
	hub := NewHub()
	first, closeFirst := tunnel(t)
	defer closeFirst()

	host := hub.Add("srv-1", "tok-1", first)
	if got, ok := hub.ByToken("tok-1"); !ok || got != host {
		t.Fatal("не нашли по токену")
	}
	if got, ok := hub.ByServer("srv-1"); !ok || got != host {
		t.Fatal("не нашли по server_id")
	}
	if _, ok := hub.ByToken("нет-такого"); ok {
		t.Fatal("нашли несуществующий токен")
	}

	second, closeSecond := tunnel(t)
	defer closeSecond()
	newHost := hub.Add("srv-1", "tok-2", second)
	if hub.Count() != 1 {
		t.Fatalf("серверов на связи: %d, ожидалась замена", hub.Count())
	}
	if _, ok := hub.ByToken("tok-1"); ok {
		t.Fatal("старый токен всё ещё действует")
	}
	if got, ok := hub.ByServer("srv-1"); !ok || got != newHost {
		t.Fatal("новый туннель не заменил старый")
	}

	hub.Remove(newHost)
	if hub.Count() != 0 {
		t.Fatalf("после Remove осталось %d", hub.Count())
	}
}

// Квота не даёт одному серверу открыть неограниченно стримов, а закрытый
// стрим возвращает место обратно.
func TestHubStreamQuota(t *testing.T) {
	hub := NewHub()
	hub.maxStreams = 2

	session, closeTunnel := tunnel(t)
	defer closeTunnel()
	host := hub.Add("srv", "tok", session)

	a, err := hub.Open(host)
	if err != nil {
		t.Fatalf("первый стрим: %v", err)
	}
	if _, err := hub.Open(host); err != nil {
		t.Fatalf("второй стрим: %v", err)
	}
	if _, err := hub.Open(host); err == nil {
		t.Fatal("третий стрим прошёл, хотя квота 2")
	}

	a.Close()
	a.Close() // повторное закрытие не должно возвращать место дважды
	if _, err := hub.Open(host); err != nil {
		t.Fatalf("место после закрытия не освободилось: %v", err)
	}
	if _, err := hub.Open(host); err == nil {
		t.Fatal("двойное закрытие вернуло лишнее место")
	}
}

// Сквозная проверка: мастер открывает стрим, хост его принимает, и сообщения
// ходят в обе стороны, не теряя границ.
func TestTunnelEndToEnd(t *testing.T) {
	masterSide, hostSide := net.Pipe()

	hostSession, err := yamux.Client(hostSide, testYamux())
	if err != nil {
		t.Fatalf("хост: %v", err)
	}
	defer hostSession.Close()

	hub := NewHub()
	masterSession, err := yamux.Server(masterSide, testYamux())
	if err != nil {
		t.Fatalf("мастер: %v", err)
	}
	defer masterSession.Close()
	host := hub.Add("srv", "tok", masterSession)

	accepted := make(chan *StreamConn, 1)
	go func() {
		stream, err := hostSession.AcceptStream()
		if err != nil {
			return
		}
		head, err := ReadFrame(stream)
		if err != nil {
			return
		}
		accepted <- NewStreamConn(stream, string(head))
	}()

	stream, err := hub.Open(host)
	if err != nil {
		t.Fatalf("открыть стрим: %v", err)
	}
	defer stream.Close()
	if err := WriteFrame(stream, []byte("9.9.9.9:1")); err != nil {
		t.Fatalf("адрес игрока: %v", err)
	}

	var conn *StreamConn
	select {
	case conn = <-accepted:
	case <-time.After(3 * time.Second):
		t.Fatal("хост не принял стрим")
	}
	defer conn.Close("тест")

	if conn.Addr() != "9.9.9.9:1" {
		t.Fatalf("адрес игрока потерялся: %q", conn.Addr())
	}

	// Игрок → хост.
	if err := WriteFrame(stream, []byte(`{"t":"cmd","d":{"kind":"привет"}}`)); err != nil {
		t.Fatalf("игрок пишет: %v", err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	env, err := conn.Read(ctx)
	if err != nil {
		t.Fatalf("хост читает: %v", err)
	}
	if env.T != proto.TCmd {
		t.Fatalf("тип: %q", env.T)
	}

	// Хост → игрок, два сообщения подряд: границы между ними должны уцелеть.
	if err := conn.SendRaw([]byte("первое")); err != nil {
		t.Fatalf("хост пишет: %v", err)
	}
	if err := conn.SendRaw([]byte("второе")); err != nil {
		t.Fatalf("хост пишет: %v", err)
	}
	for _, want := range []string{"первое", "второе"} {
		got, err := ReadFrame(stream)
		if err != nil {
			t.Fatalf("игрок читает: %v", err)
		}
		if string(got) != want {
			t.Fatalf("получено %q, ожидалось %q", got, want)
		}
	}
}

func tunnel(t *testing.T) (*yamux.Session, func()) {
	t.Helper()
	a, b := net.Pipe()
	session, err := yamux.Server(a, testYamux())
	if err != nil {
		t.Fatalf("туннель: %v", err)
	}
	// Вторая сторона нужна живой, иначе Open упрётся в мёртвый pipe.
	client, err := yamux.Client(b, testYamux())
	if err != nil {
		t.Fatalf("туннель: %v", err)
	}
	go func() {
		for {
			stream, err := client.AcceptStream()
			if err != nil {
				return
			}
			go io.Copy(io.Discard, stream)
		}
	}()
	return session, func() { session.Close(); client.Close() }
}

func testYamux() *yamux.Config {
	cfg := yamux.DefaultConfig()
	cfg.LogOutput = io.Discard
	// net.Pipe без буфера, keep-alive тут только мешает.
	cfg.EnableKeepAlive = false
	return cfg
}
