package config

type Config struct {
	Host string `toml:"host"`
	Port int    `toml:"host"`
}

func Load(path string) (*Config, error) {
	// TODO
	return nil, nil
}
