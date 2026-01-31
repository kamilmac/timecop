package config

// Config holds all application configuration
type Config struct {
	DefaultMode string // "working" or "branch"
	BaseBranch  string // "main", "master", or auto-detect

	Layout LayoutConfig
	Colors ColorConfig
}

// LayoutConfig holds layout-related settings
type LayoutConfig struct {
	DefaultRatio [2]int // left:right ratio, e.g. {30, 70}
}

// ColorConfig holds color definitions
type ColorConfig struct {
	Added           string
	Removed         string
	Context         string
	Header          string
	BorderFocused   string
	BorderUnfocused string
	StatusBar       string
}

// Default returns the default configuration
var Default = Config{
	DefaultMode: "working",
	BaseBranch:  "", // auto-detect main/master
	Layout: LayoutConfig{
		DefaultRatio: [2]int{30, 70},
	},
	Colors: ColorConfig{
		Added:           "#a6e3a1",
		Removed:         "#f38ba8",
		Context:         "#cdd6f4",
		Header:          "#89b4fa",
		BorderFocused:   "#89b4fa",
		BorderUnfocused: "#45475a",
		StatusBar:       "#313244",
	},
}
