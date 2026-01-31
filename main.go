package main

import (
	"flag"
	"fmt"
	"os"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/kmacinski/blocks/internal/app"
	"github.com/kmacinski/blocks/internal/git"
)

var (
	version = "dev"
)

func main() {
	// Parse flags
	var (
		showVersion bool
		showHelp    bool
		mode        string
		baseBranch  string
	)

	flag.BoolVar(&showVersion, "v", false, "Show version")
	flag.BoolVar(&showVersion, "version", false, "Show version")
	flag.BoolVar(&showHelp, "h", false, "Show help")
	flag.BoolVar(&showHelp, "help", false, "Show help")
	flag.StringVar(&mode, "m", "working", "Start in mode: working, branch")
	flag.StringVar(&mode, "mode", "working", "Start in mode: working, branch")
	flag.StringVar(&baseBranch, "b", "", "Base branch for branch mode")
	flag.StringVar(&baseBranch, "base", "", "Base branch for branch mode")
	flag.Parse()

	if showVersion {
		fmt.Printf("blocks %s\n", version)
		os.Exit(0)
	}

	if showHelp {
		printHelp()
		os.Exit(0)
	}

	// Change to target directory if specified
	args := flag.Args()
	if len(args) > 0 {
		if err := os.Chdir(args[0]); err != nil {
			fmt.Fprintf(os.Stderr, "Error: cannot change to directory %s: %v\n", args[0], err)
			os.Exit(1)
		}
	}

	// Create git client
	gitClient := git.NewClient(baseBranch)

	// Create and run app
	application := app.New(gitClient)

	// Set initial mode if specified
	if mode == "branch" {
		application.Init() // This will be overridden, but needed for state
	}

	p := tea.NewProgram(
		application,
		tea.WithAltScreen(),
		tea.WithMouseCellMotion(),
	)

	if _, err := p.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func printHelp() {
	fmt.Println(`blocks - AI-Native IDE

A read-first terminal IDE for AI-driven development workflows.

Usage:
  blocks [flags] [path]

Arguments:
  path              Target directory (default: current dir)

Flags:
  -m, --mode        Start in mode: working, branch (default: working)
  -b, --base        Base branch for branch mode (default: auto-detect)
  -h, --help        Show help
  -v, --version     Show version

Keybindings:
  j/k               Navigate up/down
  h/l               Switch window
  Tab               Cycle windows
  Ctrl+d/u          Scroll half page
  g/G               Go to top/bottom
  1                 Working diff mode
  2                 Branch diff mode
  y                 Copy file path
  o                 Open in $EDITOR
  r                 Refresh
  ?                 Toggle help
  q                 Quit`)
}
