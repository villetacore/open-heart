package auth

import (
	"crypto/ed25519"
	"crypto/rand"
	"testing"
	"time"
)

func TestPasswordHashRoundTrip(t *testing.T) {
	hash, err := HashPassword("правильный-конь")
	if err != nil {
		t.Fatal(err)
	}
	if !VerifyPassword(hash, "правильный-конь") {
		t.Error("верный пароль не принят")
	}
	if VerifyPassword(hash, "неверный") {
		t.Error("неверный пароль принят")
	}
	// Соль случайна: два хеша одного пароля обязаны отличаться.
	other, err := HashPassword("правильный-конь")
	if err != nil {
		t.Fatal(err)
	}
	if other == hash {
		t.Error("хеши совпали — соль не работает")
	}
}

func TestShortPasswordRejected(t *testing.T) {
	if _, err := HashPassword("12345"); err == nil {
		t.Error("короткий пароль должен отвергаться")
	}
}

func TestTicketLifecycle(t *testing.T) {
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	now := time.Now()
	token, err := Sign(priv, Ticket{
		AccountID: 42, Nickname: "vasya", Aud: "srv1", Exp: now.Add(TicketTTL).Unix(),
	})
	if err != nil {
		t.Fatal(err)
	}

	ticket, err := Verify(pub, token, "srv1", now)
	if err != nil {
		t.Fatalf("свой тикет не прошёл проверку: %v", err)
	}
	if ticket.AccountID != 42 || ticket.Nickname != "vasya" {
		t.Fatalf("тикет разобрался неверно: %+v", ticket)
	}
	if ticket.Nonce == "" {
		t.Error("nonce не проставлен")
	}

	// Тикет для одного сервера не годится другому.
	if _, err := Verify(pub, token, "srv2", now); err == nil {
		t.Error("тикет принят чужим сервером")
	}
	// И протухает.
	if _, err := Verify(pub, token, "srv1", now.Add(2*TicketTTL)); err == nil {
		t.Error("истёкший тикет принят")
	}
	// Подпись чужим ключом не проходит.
	otherPub, _, _ := ed25519.GenerateKey(rand.Reader)
	if _, err := Verify(otherPub, token, "srv1", now); err == nil {
		t.Error("тикет принят с чужим ключом")
	}
}

func TestTamperedTicketRejected(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(rand.Reader)
	token, err := Sign(priv, Ticket{AccountID: 1, Aud: "srv1", Exp: time.Now().Add(time.Minute).Unix()})
	if err != nil {
		t.Fatal(err)
	}
	broken := []byte(token)
	broken[3] ^= 0x01
	if _, err := Verify(pub, string(broken), "srv1", time.Now()); err == nil {
		t.Error("подделанный тикет принят")
	}
}

func TestPublicKeyRoundTrip(t *testing.T) {
	_, priv, _ := ed25519.GenerateKey(rand.Reader)
	pub, err := ParsePublicKey(PublicKeyString(priv))
	if err != nil {
		t.Fatal(err)
	}
	if !pub.Equal(priv.Public()) {
		t.Error("публичный ключ не совпал с приватным")
	}
}
