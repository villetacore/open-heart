// Package proto — сообщения сетевого протокола OpenHeart.
//
// Единственный источник правды по формату — protocol/schema.md в корне репозитория.
// Любое несовместимое изменение здесь = инкремент Version там и тут.
package proto

import "encoding/json"

// Version — версия протокола, проверяется в hello.
const Version = 1

// Теги сообщений (поле "t" конверта).
const (
	// клиент → сервер
	THello    = "hello"
	TInput    = "input"
	TFire     = "fire"
	THitClaim = "hit"
	TInteract = "interact"
	TCmd      = "cmd"
	TChat     = "chat"
	TPing     = "ping"

	// сервер → клиент
	TWelcome  = "welcome"
	TSnapshot = "snap"
	TEvent    = "event"
	TReject   = "reject"
	TPong     = "pong"
)

// Envelope — обёртка любого сообщения.
type Envelope struct {
	T string          `json:"t"`
	D json.RawMessage `json:"d,omitempty"`
}

// Into разбирает тело конверта в v.
func (e Envelope) Into(v any) error {
	if len(e.D) == 0 {
		return nil
	}
	return json.Unmarshal(e.D, v)
}

// Marshal собирает конверт с телом d.
func Marshal(t string, d any) ([]byte, error) {
	env := Envelope{T: t}
	if d != nil {
		body, err := json.Marshal(d)
		if err != nil {
			return nil, err
		}
		env.D = body
	}
	return json.Marshal(env)
}

// Vec3 — мировая позиция/направление.
type Vec3 [3]float32

// ── клиент → сервер ──────────────────────────────────────────────────────────

// Auth — способ входа, объявленный клиентом (см. auth_mode сервера).
type Auth struct {
	Mode     string `json:"mode"` // master | local | open
	Ticket   string `json:"ticket,omitempty"`
	Login    string `json:"login,omitempty"`
	Password string `json:"password,omitempty"`
	Nickname string `json:"nickname,omitempty"`
	Register bool   `json:"register,omitempty"` // local: создать аккаунт при первом входе
}

type Hello struct {
	Protocol    int    `json:"protocol"`
	GameVersion string `json:"game_version"`
	PresetID    string `json:"preset_id"`
	ContentHash string `json:"content_hash"`
	Password    string `json:"password,omitempty"` // пароль на вход на сервер
	Auth        Auth   `json:"auth"`
}

// Биты поля Input.Buttons.
const (
	BtnJump uint16 = 1 << iota
	BtnSprint
	BtnFire
	BtnAltFire
	BtnReload
	BtnUse
	BtnCrouch
)

type Input struct {
	Tick    uint32     `json:"tick"`
	Move    [2]float32 `json:"move"`
	Yaw     float32    `json:"yaw"`
	Pitch   float32    `json:"pitch"`
	Buttons uint16     `json:"buttons"`
	Pos     Vec3       `json:"pos"`
	Vel     Vec3       `json:"vel"`
	OnFloor bool       `json:"on_floor"`
}

type Fire struct {
	Tick      uint32 `json:"tick"`
	Weapon    string `json:"weapon"`
	Origin    Vec3   `json:"origin"`
	Dir       Vec3   `json:"dir"`
	Secondary bool   `json:"secondary"`
}

type HitClaim struct {
	Tick   uint32 `json:"tick"`
	Weapon string `json:"weapon"`
	Target uint16 `json:"target"`
	Pos    Vec3   `json:"pos"`
	Part   uint8  `json:"part"`
}

type Interact struct {
	Target uint16 `json:"target"`
	Kind   string `json:"kind"`
}

// Значения Cmd.Kind.
const (
	CmdEnterDelve      = "enter_delve"
	CmdLeaveDelve      = "leave_delve"
	CmdSwitchWeapon    = "switch_weapon"
	CmdReload          = "reload"
	CmdTakePerk        = "take_perk"
	CmdEquip           = "equip"
	CmdUseItem         = "use_item"
	CmdRespawn         = "respawn"
	CmdRevive          = "revive"
	CmdSelectCharacter = "select_character"
)

type Cmd struct {
	Kind string          `json:"kind"`
	Args json.RawMessage `json:"args,omitempty"`
}

type Chat struct {
	Text string `json:"text"`
}

type Ping struct {
	T int64 `json:"t"`
}

// ── сервер → клиент ──────────────────────────────────────────────────────────

type World struct {
	Kind  string `json:"kind"` // hub | delve
	Seed  uint64 `json:"seed"`
	Depth int    `json:"depth"`
}

type PlayerInfo struct {
	Peer     uint16 `json:"peer"`
	Nickname string `json:"nickname"`
	Class    string `json:"class,omitempty"`
	Level    int    `json:"level,omitempty"`
}

type Welcome struct {
	Peer        uint16       `json:"peer"`
	Tick        uint32       `json:"tick"`
	PresetID    string       `json:"preset_id"`
	ContentHash string       `json:"content_hash"`
	World       World        `json:"world"`
	Players     []PlayerInfo `json:"players"`
}

// Значения Ent.Kind.
const (
	KindPlayer uint8 = iota
	KindEnemy
	KindProjectile
	KindItem
	KindNPC
)

// Биты Ent.Flags.
const (
	FlagDowned uint8 = 1 << iota
	FlagStunned
	FlagBurning
	FlagElite
	FlagFiring
	FlagHidden
)

// Ent — запись сущности в снапшоте.
type Ent struct {
	ID   uint16 `json:"id"`
	Kind uint8  `json:"kind"`
	// Индекс вида врага в enemies.json пресета (+1); 0 — не задан.
	TypeID uint16 `json:"t,omitempty"`
	// Кому принадлежит сущность (лут); 0 — общая.
	Owner uint16  `json:"owner,omitempty"`
	Pos   Vec3    `json:"pos"`
	Yaw   float32 `json:"yaw"`
	HP    uint8   `json:"hp"` // процент от максимума
	Flags uint8   `json:"flags"`
}

type Snapshot struct {
	Tick        uint32 `json:"tick"`
	Ack         uint32 `json:"ack"` // последний учтённый Input.Tick этого клиента
	Full        bool   `json:"full,omitempty"`
	Players     []Ent  `json:"players,omitempty"`
	Enemies     []Ent  `json:"enemies,omitempty"`
	Projectiles []Ent  `json:"projectiles,omitempty"`
	Items       []Ent  `json:"items,omitempty"`
}

// Значения Event.Kind.
const (
	EvDamage    = "damage"
	EvEnemyDied = "enemy_died"
	EvLoot      = "loot"
	EvXP        = "xp"
	EvLevelUp   = "level_up"
	EvQuest     = "quest"
	EvDowned    = "downed"
	EvRevived   = "revived"
	EvPlayerOut = "player_out"
	EvWipe      = "wipe"
	EvJoin      = "join"
	EvLeave     = "leave"
	EvChat      = "chat"
	EvWorldLoad = "world_load"
	EvDespawn   = "despawn"
	EvPickup    = "pickup"
	EvNotice    = "notice"
)

type Event struct {
	Kind   string          `json:"kind"`
	Actor  uint16          `json:"actor,omitempty"`
	Target uint16          `json:"target,omitempty"`
	Amount float32         `json:"amount,omitempty"`
	Text   string          `json:"text,omitempty"`
	Pos    *Vec3           `json:"pos,omitempty"`
	Extra  json.RawMessage `json:"extra,omitempty"`
}

// Причины отказа (Reject.Reason).
const (
	RejProtocol   = "protocol"
	RejVersion    = "version"
	RejContent    = "content"
	RejAuth       = "auth"
	RejBanned     = "banned"
	RejFull       = "full"
	RejBadRequest = "bad_request"
	RejTooBig     = "too_big"
	RejRate       = "rate"
)

type Reject struct {
	Reason string `json:"reason"`
	Detail string `json:"detail,omitempty"`
}

type Pong struct {
	T int64 `json:"t"`
}
