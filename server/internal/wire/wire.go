// Package wire — транспортный слой: WebSocket-соединение, кодек конвертов, помпа записи.
//
// Читает вызывающий (обычно в своей горутине), пишет — внутренняя помпа через буфер:
// медленный клиент не тормозит игровой тик, а отваливается по переполнению очереди.
package wire

import (
	"context"
	"encoding/json"
	"errors"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/coder/websocket"

	"github.com/villetacore/open-heart/server/internal/proto"
)

const (
	// ReadLimit — максимальный размер входящего кадра (см. protocol/schema.md).
	ReadLimit  = 64 * 1024
	writeQueue = 128
	writeWait  = 5 * time.Second
)

// ErrBacklog — клиент не успевает читать, очередь отправки переполнена.
var ErrBacklog = errors.New("wire: очередь отправки переполнена")

// Conn — соединение с игроком. Реализаций две: прямое соединение по
// WebSocket ([`WSConn`]) и туннель через релей мастера
// (`internal/relay.StreamConn`). Комната разницы не видит.
type Conn interface {
	// Addr — адрес пира для логов и рейт-лимитов.
	Addr() string
	// Read ждёт следующий конверт.
	Read(ctx context.Context) (proto.Envelope, error)
	// Send ставит сообщение в очередь отправки. Не блокирует.
	Send(tag string, d any) error
	// SendRaw отправляет готовый кадр (снапшот сериализуется один раз на комнату).
	SendRaw(data []byte) error
	// Reject сообщает причину отказа и закрывает соединение.
	Reject(reason, detail string)
	// Close закрывает соединение (идемпотентно).
	Close(reason string)
	// Done закрывается, когда соединение закрыто.
	Done() <-chan struct{}
}

// WSConn — прямое соединение по WebSocket.
type WSConn struct {
	ws   *websocket.Conn
	out  chan []byte
	addr string

	once   sync.Once
	closed chan struct{}
}

// Accept поднимает WebSocket поверх HTTP-запроса.
func Accept(w http.ResponseWriter, r *http.Request) (*WSConn, error) {
	ws, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		// Игровой клиент — не браузер; Origin не проверяем, доступ режет auth.
		InsecureSkipVerify: true,
		CompressionMode:    websocket.CompressionDisabled,
	})
	if err != nil {
		return nil, err
	}
	return newConn(ws, RemoteAddr(r)), nil
}

// Dial подключается к серверу (используется клиентскими инструментами и ботами).
func Dial(ctx context.Context, url string) (*WSConn, error) {
	ws, _, err := websocket.Dial(ctx, url, nil)
	if err != nil {
		return nil, err
	}
	return newConn(ws, url), nil
}

func newConn(ws *websocket.Conn, addr string) *WSConn {
	ws.SetReadLimit(ReadLimit)
	c := &WSConn{
		ws:     ws,
		out:    make(chan []byte, writeQueue),
		addr:   addr,
		closed: make(chan struct{}),
	}
	go c.writePump()
	return c
}

// Addr — адрес пира для логов и рейт-лимитов.
func (c *WSConn) Addr() string { return c.addr }

// Read ждёт следующий конверт.
func (c *WSConn) Read(ctx context.Context) (proto.Envelope, error) {
	var env proto.Envelope
	_, data, err := c.ws.Read(ctx)
	if err != nil {
		return env, err
	}
	if err := json.Unmarshal(data, &env); err != nil {
		return env, err
	}
	return env, nil
}

// Send ставит сообщение в очередь отправки. Не блокирует.
func (c *WSConn) Send(t string, d any) error {
	data, err := proto.Marshal(t, d)
	if err != nil {
		return err
	}
	return c.SendRaw(data)
}

// SendRaw отправляет готовый кадр (используется для широковещательных снапшотов,
// сериализованных один раз на всю комнату).
func (c *WSConn) SendRaw(data []byte) error {
	select {
	case <-c.closed:
		return net.ErrClosed
	case c.out <- data:
		return nil
	default:
		c.Close("backlog")
		return ErrBacklog
	}
}

// Reject отправляет отказ и закрывает соединение.
func (c *WSConn) Reject(reason, detail string) {
	_ = c.Send(proto.TReject, proto.Reject{Reason: reason, Detail: detail})
	// даём помпе шанс дописать кадр перед закрытием
	time.AfterFunc(200*time.Millisecond, func() { c.Close(reason) })
}

// Close закрывает соединение (идемпотентно).
func (c *WSConn) Close(reason string) {
	c.once.Do(func() {
		close(c.closed)
		_ = c.ws.Close(websocket.StatusNormalClosure, truncate(reason, 100))
	})
}

// Done закрывается, когда соединение закрыто.
func (c *WSConn) Done() <-chan struct{} { return c.closed }

func (c *WSConn) writePump() {
	for {
		select {
		case <-c.closed:
			return
		case data := <-c.out:
			ctx, cancel := context.WithTimeout(context.Background(), writeWait)
			err := c.ws.Write(ctx, websocket.MessageText, data)
			cancel()
			if err != nil {
				c.Close("write")
				return
			}
		}
	}
}

// RemoteAddr — IP клиента с учётом обратного прокси (Caddy ставит X-Forwarded-For).
func RemoteAddr(r *http.Request) string {
	if fwd := r.Header.Get("X-Forwarded-For"); fwd != "" {
		if i := strings.IndexByte(fwd, ','); i > 0 {
			return strings.TrimSpace(fwd[:i])
		}
		return strings.TrimSpace(fwd)
	}
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return r.RemoteAddr
	}
	return host
}

func truncate(s string, n int) string {
	if len(s) <= n {
		return s
	}
	return s[:n]
}
