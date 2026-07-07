package main

import (
	"fmt"
	"os"

	"github.com/quanttide/qtcloud-devops-provider/internal/config"
	"github.com/quanttide/qtcloud-devops-provider/internal/version"
)

func main() {
	cfg := config.Load()
	fmt.Printf("qtcloud-devops-provider %s\n", version.Version)
	fmt.Printf("store: %s (%s)\n", cfg.StorePath, cfg.StoreDriver)
	os.Exit(0)
}
