// Package content — идентификация набора данных пресета.
//
// Пресет и есть игра: при расхождении JSON у игроков разный баланс, поэтому
// сервер и клиент сверяют content_hash (docs/MULTIPLAYER.md §8.1).
package content

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// Hash считает устойчивый хеш пресета: sha256 по отсортированным парам
// (относительный путь, содержимое) всех *.json и *.toml внутри каталога.
// Возвращает "" если каталога нет — тогда сверка content_hash отключается.
func Hash(presetDir string) (string, error) {
	info, err := os.Stat(presetDir)
	if os.IsNotExist(err) {
		return "", nil
	}
	if err != nil {
		return "", err
	}
	if !info.IsDir() {
		return "", fmt.Errorf("content: %s не каталог", presetDir)
	}

	var files []string
	err = filepath.WalkDir(presetDir, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		switch strings.ToLower(filepath.Ext(path)) {
		case ".json", ".toml", ".ron":
			rel, err := filepath.Rel(presetDir, path)
			if err != nil {
				return err
			}
			files = append(files, filepath.ToSlash(rel))
		}
		return nil
	})
	if err != nil {
		return "", err
	}
	sort.Strings(files)

	sum := sha256.New()
	for _, rel := range files {
		fmt.Fprintf(sum, "%s\n", rel)
		f, err := os.Open(filepath.Join(presetDir, filepath.FromSlash(rel)))
		if err != nil {
			return "", err
		}
		// Переводы строк нормализуем: git на Windows не должен менять хеш.
		if err := copyNormalized(sum, f); err != nil {
			f.Close()
			return "", err
		}
		f.Close()
	}
	return hex.EncodeToString(sum.Sum(nil)), nil
}

func copyNormalized(dst io.Writer, src io.Reader) error {
	buf := make([]byte, 32*1024)
	out := make([]byte, 0, len(buf))
	for {
		n, err := src.Read(buf)
		if n > 0 {
			out = out[:0]
			for _, b := range buf[:n] {
				if b == '\r' {
					continue
				}
				out = append(out, b)
			}
			if _, werr := dst.Write(out); werr != nil {
				return werr
			}
		}
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
	}
}
