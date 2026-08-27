package relay

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net"
	"sync"
	"time"

	"github.com/villetacore/open-heart/server/internal/proto"
)

const (
	writeQueue = 128
	writeWait  = 5 * time.Second
)

// ErrBacklog — игрок не успевает читать: очередь отправки переполнена.
var ErrBacklog = errors.New("relay: очередь отправки переполнена")

// StreamConn — соединение с игроком поверх стрима туннеля.
//
// Снаружи выглядит ровно как прямое соединение (`wire.Conn`): комната не должна
// знать, пришёл игрок напрямую или через релей.
type StreamConn struct {
	stream io.ReadWriteCloser
	out    chan []byte
	addr   string

	once   sync.Once
	closed chan struct{}
}

// NewStreamConn оборачивает стрим туннеля.
//
// `addr` — адрес игрока, каким его видел мастер: он нужен в логах и
// рейт-лимитах, а через туннель настоящий адрес иначе не узнать.
func NewStreamConn(stream io.ReadWriteCloser, addr string) *StreamConn {
	c := &StreamConn{
		stream: stream,
		out:    make(chan []byte, writeQueue),
		addr:   addr,
		closed: make(chan struct{}),
	}
	go c.writePump()
	return c
}

func (c *StreamConn) Addr() string { return c.addr }

func (c *StreamConn) Read(ctx context.Context) (proto.Envelope, error) {
	var envelope proto.Envelope

	// У стрима нет контекста: ставим дедлайн, если он поддерживается.
	if deadline, ok := ctx.Deadline(); ok {
		if withDeadline, ok := c.stream.(interface{ SetReadDeadline(time.Time) error }); ok {
			_ = withDeadline.SetReadDeadline(deadline)
		}
	}

	payload, err := ReadFrame(c.stream)
	if err != nil {
		return envelope, err
	}
	if err := json.Unmarshal(payload, &envelope); err != nil {
		return envelope, err
	}
	return envelope, nil
}

func (c *StreamConn) Send(tag string, d any) error {
	data, err := proto.Marshal(tag, d)
	if err != nil {
		return err
	}
	return c.SendRaw(data)
}

func (c *StreamConn) SendRaw(data []byte) error {
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

func (c *StreamConn) Reject(reason, detail string) {
	_ = c.Send(proto.TReject, proto.Reject{Reason: reason, Detail: detail})
	// Даём помпе дописать кадр, прежде чем рвать стрим.
	time.AfterFunc(200*time.Millisecond, func() { c.Close(reason) })
}

func (c *StreamConn) Close(reason string) {
	c.once.Do(func() {
		close(c.closed)
		_ = c.stream.Close()
	})
}

func (c *StreamConn) Done() <-chan struct{} { return c.closed }

func (c *StreamConn) writePump() {
	for {
		select {
		case <-c.closed:
			return
		case data := <-c.out:
			if withDeadline, ok := c.stream.(interface{ SetWriteDeadline(time.Time) error }); ok {
				_ = withDeadline.SetWriteDeadline(time.Now().Add(writeWait))
			}
			if err := WriteFrame(c.stream, data); err != nil {
				c.Close("write")
				return
			}
		}
	}
}
