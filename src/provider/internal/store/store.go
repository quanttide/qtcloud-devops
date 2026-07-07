package store

type Store interface {
	Read(key string) ([]byte, error)
	Write(key string, data []byte) error
	List(prefix string) ([]string, error)
	Delete(key string) error
}
