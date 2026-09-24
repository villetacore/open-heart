// oh-content-hash prints the server's preset fingerprint for cross-language checks.
package main

import (
	"fmt"
	"github.com/villetacore/open-heart/server/internal/content"
	"os"
)

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: oh-content-hash <preset-directory>")
		os.Exit(2)
	}
	hash, err := content.Hash(os.Args[1])
	if err != nil || hash == "" {
		fmt.Fprintln(os.Stderr, "cannot hash preset:", err)
		os.Exit(1)
	}
	fmt.Println(hash)
}
