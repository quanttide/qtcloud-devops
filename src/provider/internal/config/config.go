package config

import "os"

type Config struct {
	Addr        string
	StorePath   string
	StoreDriver string
	JWTSecret   string
	AdminPass   string
	LogLevel    string
	LogFormat   string
}

func Load() *Config {
	return &Config{
		Addr:        env("ADDR", ":8000"),
		StorePath:   env("STORE_PATH", "data"),
		StoreDriver: env("STORE_DRIVER", "file"),
		JWTSecret:   os.Getenv("JWT_SECRET"),
		AdminPass:   os.Getenv("ADMIN_PASSWORD"),
		LogLevel:    env("LOG_LEVEL", "info"),
		LogFormat:   env("LOG_FORMAT", "text"),
	}
}

func env(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
