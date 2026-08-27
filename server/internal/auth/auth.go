// Package auth — пароли (argon2id) и подписанные тикеты входа (ed25519).
//
// Тикет — способ пустить игрока на чужой игровой сервер, не показывая тому пароль
// от мастер-аккаунта: сервер проверяет подпись публичным ключом мастера офлайн.
// Подробнее — docs/MULTIPLAYER.md §10.
package auth

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"
	"time"

	"golang.org/x/crypto/argon2"
)

// Параметры argon2id. Меняются только вместе с версией в кодировке хеша.
const (
	argonTime    = 3
	argonMemory  = 64 * 1024 // КиБ
	argonThreads = 4
	argonKeyLen  = 32
	saltLen      = 16
)

// TicketTTL — время жизни тикета входа. Короткое: украденный бесполезен.
const TicketTTL = 60 * time.Second

var (
	ErrBadPassword = errors.New("auth: неверный логин или пароль")
	ErrBadTicket   = errors.New("auth: тикет не проходит проверку")
)

// HashPassword возвращает кодированный хеш argon2id.
func HashPassword(password string) (string, error) {
	if len(password) < 6 {
		return "", errors.New("auth: пароль короче 6 символов")
	}
	salt := make([]byte, saltLen)
	if _, err := rand.Read(salt); err != nil {
		return "", err
	}
	key := argon2.IDKey([]byte(password), salt, argonTime, argonMemory, argonThreads, argonKeyLen)
	b64 := base64.RawStdEncoding
	return fmt.Sprintf("$argon2id$v=%d$m=%d,t=%d,p=%d$%s$%s",
		argon2.Version, argonMemory, argonTime, argonThreads,
		b64.EncodeToString(salt), b64.EncodeToString(key)), nil
}

// VerifyPassword проверяет пароль против кодированного хеша.
func VerifyPassword(encoded, password string) bool {
	parts := strings.Split(encoded, "$")
	if len(parts) != 6 || parts[1] != "argon2id" {
		return false
	}
	var version int
	if _, err := fmt.Sscanf(parts[2], "v=%d", &version); err != nil || version != argon2.Version {
		return false
	}
	var memory, times uint32
	var threads uint8
	if _, err := fmt.Sscanf(parts[3], "m=%d,t=%d,p=%d", &memory, &times, &threads); err != nil {
		return false
	}
	b64 := base64.RawStdEncoding
	salt, err := b64.DecodeString(parts[4])
	if err != nil {
		return false
	}
	want, err := b64.DecodeString(parts[5])
	if err != nil {
		return false
	}
	got := argon2.IDKey([]byte(password), salt, times, memory, threads, uint32(len(want)))
	return subtle.ConstantTimeCompare(got, want) == 1
}

// Ticket — одноразовый пропуск на конкретный игровой сервер.
type Ticket struct {
	AccountID int64  `json:"account_id"`
	Nickname  string `json:"nickname"`
	Aud       string `json:"aud"` // server_id, кому предназначен
	Exp       int64  `json:"exp"` // unix-секунды
	Nonce     string `json:"nonce"`
}

// Sign кодирует тикет в строку `base64(json).base64(signature)`.
func Sign(priv ed25519.PrivateKey, t Ticket) (string, error) {
	if t.Nonce == "" {
		nonce := make([]byte, 12)
		if _, err := rand.Read(nonce); err != nil {
			return "", err
		}
		t.Nonce = base64.RawURLEncoding.EncodeToString(nonce)
	}
	body, err := json.Marshal(t)
	if err != nil {
		return "", err
	}
	sig := ed25519.Sign(priv, body)
	b64 := base64.RawURLEncoding
	return b64.EncodeToString(body) + "." + b64.EncodeToString(sig), nil
}

// Verify проверяет подпись, срок и адресата тикета.
func Verify(pub ed25519.PublicKey, token, audience string, now time.Time) (*Ticket, error) {
	rawBody, rawSig, ok := strings.Cut(token, ".")
	if !ok {
		return nil, ErrBadTicket
	}
	b64 := base64.RawURLEncoding
	body, err := b64.DecodeString(rawBody)
	if err != nil {
		return nil, ErrBadTicket
	}
	sig, err := b64.DecodeString(rawSig)
	if err != nil {
		return nil, ErrBadTicket
	}
	if !ed25519.Verify(pub, body, sig) {
		return nil, ErrBadTicket
	}
	var t Ticket
	if err := json.Unmarshal(body, &t); err != nil {
		return nil, ErrBadTicket
	}
	if now.Unix() > t.Exp {
		return nil, fmt.Errorf("%w: истёк", ErrBadTicket)
	}
	if audience != "" && t.Aud != audience {
		return nil, fmt.Errorf("%w: чужой адресат %q", ErrBadTicket, t.Aud)
	}
	return &t, nil
}

// LoadOrCreateKey читает ed25519-ключ мастера, создавая его при первом запуске.
func LoadOrCreateKey(path string) (ed25519.PrivateKey, error) {
	data, err := os.ReadFile(path)
	switch {
	case err == nil:
		raw, err := base64.StdEncoding.DecodeString(strings.TrimSpace(string(data)))
		if err != nil {
			return nil, fmt.Errorf("ключ %s повреждён: %w", path, err)
		}
		if len(raw) != ed25519.PrivateKeySize {
			return nil, fmt.Errorf("ключ %s повреждён: длина %d", path, len(raw))
		}
		return ed25519.PrivateKey(raw), nil
	case errors.Is(err, os.ErrNotExist):
		_, priv, err := ed25519.GenerateKey(rand.Reader)
		if err != nil {
			return nil, err
		}
		enc := base64.StdEncoding.EncodeToString(priv)
		if err := os.WriteFile(path, []byte(enc+"\n"), 0o600); err != nil {
			return nil, err
		}
		return priv, nil
	default:
		return nil, err
	}
}

// PublicKeyString — публичный ключ в base64 для раздачи серверам.
func PublicKeyString(priv ed25519.PrivateKey) string {
	return base64.StdEncoding.EncodeToString(priv.Public().(ed25519.PublicKey))
}

// ParsePublicKey разбирает публичный ключ мастера, полученный по /v1/auth/pubkey.
func ParsePublicKey(s string) (ed25519.PublicKey, error) {
	raw, err := base64.StdEncoding.DecodeString(strings.TrimSpace(s))
	if err != nil {
		return nil, err
	}
	if len(raw) != ed25519.PublicKeySize {
		return nil, fmt.Errorf("auth: публичный ключ длины %d", len(raw))
	}
	return ed25519.PublicKey(raw), nil
}

// RandomToken — случайный непредсказуемый токен (refresh, relay, секрет сервера).
func RandomToken(n int) (string, error) {
	buf := make([]byte, n)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(buf), nil
}
