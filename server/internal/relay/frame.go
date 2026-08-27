// Package relay — туннель для серверов без белого IP.
//
// Хост держит одно исходящее соединение к мастеру, поверх него поднят
// мультиплексор (yamux): каждый входящий игрок — отдельный стрим. Мастер
// перекладывает байты между игроком (WebSocket) и стримом хоста
// (docs/MULTIPLAYER.md §8.3).
//
// Внутри стрима сообщения идут кадрами с длиной: стрим байтовый, а игровой
// протокол — сообщения, и границы между ними терять нельзя.
package relay

import (
	"encoding/binary"
	"errors"
	"fmt"
	"io"
)

// MaxFrame — предел размера одного сообщения. Совпадает с лимитом WebSocket
// (`wire.ReadLimit`): туннель не должен пропускать то, что не пропустил бы
// прямой канал.
const MaxFrame = 64 * 1024

// ErrFrameTooBig — кадр больше допустимого: соединение после этого рвём,
// потому что доверять границам сообщений в потоке уже нельзя.
var ErrFrameTooBig = errors.New("relay: кадр больше допустимого")

// WriteFrame отправляет одно сообщение: 4 байта длины и тело.
func WriteFrame(w io.Writer, payload []byte) error {
	if len(payload) > MaxFrame {
		return fmt.Errorf("%w: %d байт", ErrFrameTooBig, len(payload))
	}
	var header [4]byte
	binary.BigEndian.PutUint32(header[:], uint32(len(payload)))
	if _, err := w.Write(header[:]); err != nil {
		return err
	}
	_, err := w.Write(payload)
	return err
}

// ReadFrame читает одно сообщение целиком.
func ReadFrame(r io.Reader) ([]byte, error) {
	var header [4]byte
	if _, err := io.ReadFull(r, header[:]); err != nil {
		return nil, err
	}
	size := binary.BigEndian.Uint32(header[:])
	if size > MaxFrame {
		return nil, fmt.Errorf("%w: %d байт", ErrFrameTooBig, size)
	}
	payload := make([]byte, size)
	if _, err := io.ReadFull(r, payload); err != nil {
		return nil, err
	}
	return payload, nil
}
