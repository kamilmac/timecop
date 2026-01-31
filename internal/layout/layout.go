package layout

import (
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/window"
)

// Direction represents the split direction
type Direction int

const (
	Horizontal Direction = iota
	Vertical
)

// Slot represents a named area in the layout
type Slot struct {
	Name      string
	Direction Direction
	Children  []Slot
	Ratios    []int
}

// Layout defines the structure of slots
type Layout struct {
	Name      string
	Direction Direction
	Slots     []Slot
	Ratios    []int
}

// Predefined layouts
var (
	TwoColumn = Layout{
		Name:      "two-column",
		Direction: Horizontal,
		Ratios:    []int{30, 70},
		Slots: []Slot{
			{Name: "left"},
			{Name: "right"},
		},
	}

	ThreeSlot = Layout{
		Name:      "three-slot",
		Direction: Horizontal,
		Ratios:    []int{30, 70},
		Slots: []Slot{
			{
				Name:      "left",
				Direction: Vertical,
				Ratios:    []int{60, 40},
				Children: []Slot{
					{Name: "left-top"},
					{Name: "left-bottom"},
				},
			},
			{Name: "right"},
		},
	}

	Stacked = Layout{
		Name:      "stacked",
		Direction: Vertical,
		Ratios:    []int{30, 70},
		Slots: []Slot{
			{Name: "top"},
			{Name: "bottom"},
		},
	}
)

// Breakpoint defines when to switch layouts
type Breakpoint struct {
	MinWidth int
	Layout   Layout
}

// ResponsiveConfig defines breakpoints for responsive layouts
type ResponsiveConfig struct {
	Breakpoints []Breakpoint
}

// DefaultResponsive is the default responsive configuration
var DefaultResponsive = ResponsiveConfig{
	Breakpoints: []Breakpoint{
		{MinWidth: 120, Layout: ThreeSlot},
		{MinWidth: 80, Layout: TwoColumn},
		{MinWidth: 0, Layout: Stacked},
	},
}

// GetLayout returns the appropriate layout for the given width
func (r *ResponsiveConfig) GetLayout(width int) Layout {
	for _, bp := range r.Breakpoints {
		if width >= bp.MinWidth {
			return bp.Layout
		}
	}
	return TwoColumn
}

// Manager handles layout rendering
type Manager struct {
	responsive ResponsiveConfig
	current    Layout
	width      int
	height     int
}

// NewManager creates a new layout manager
func NewManager(responsive ResponsiveConfig) *Manager {
	return &Manager{
		responsive: responsive,
		current:    TwoColumn,
	}
}

// Resize updates the layout dimensions
func (m *Manager) Resize(width, height int) {
	m.width = width
	m.height = height
	m.current = m.responsive.GetLayout(width)
}

// CurrentLayout returns the current layout
func (m *Manager) CurrentLayout() Layout {
	return m.current
}

// Render renders all windows according to the layout
func (m *Manager) Render(windows map[string]window.Window, assignments map[string]string, statusBar string) string {
	if m.width == 0 || m.height == 0 {
		return ""
	}

	// Reserve space for status bar
	contentHeight := m.height - 1

	// Render the layout
	content := m.renderSlots(m.current.Slots, m.current.Ratios, m.current.Direction, m.width, contentHeight, windows, assignments)

	// Add status bar at the bottom
	return lipgloss.JoinVertical(lipgloss.Left, content, statusBar)
}

func (m *Manager) renderSlots(slots []Slot, ratios []int, dir Direction, width, height int, windows map[string]window.Window, assignments map[string]string) string {
	if len(slots) == 0 {
		return ""
	}

	// Calculate dimensions for each slot
	sizes := calculateSizes(ratios, width, height, dir)

	var rendered []string
	for i, slot := range slots {
		var slotWidth, slotHeight int
		if dir == Horizontal {
			slotWidth = sizes[i]
			slotHeight = height
		} else {
			slotWidth = width
			slotHeight = sizes[i]
		}

		var content string
		if len(slot.Children) > 0 {
			// Recursive render for nested slots
			content = m.renderSlots(slot.Children, slot.Ratios, slot.Direction, slotWidth, slotHeight, windows, assignments)
		} else {
			// Render the window assigned to this slot
			windowName := assignments[slot.Name]
			if w, ok := windows[windowName]; ok {
				content = w.View(slotWidth, slotHeight)
			} else {
				content = strings.Repeat(" ", slotWidth)
			}
		}

		rendered = append(rendered, content)
	}

	// Join the rendered slots
	if dir == Horizontal {
		return lipgloss.JoinHorizontal(lipgloss.Top, rendered...)
	}
	return lipgloss.JoinVertical(lipgloss.Left, rendered...)
}

func calculateSizes(ratios []int, width, height int, dir Direction) []int {
	total := 0
	for _, r := range ratios {
		total += r
	}

	var dimension int
	if dir == Horizontal {
		dimension = width
	} else {
		dimension = height
	}

	sizes := make([]int, len(ratios))
	remaining := dimension
	for i, r := range ratios {
		if i == len(ratios)-1 {
			// Last slot gets remaining space to avoid rounding issues
			sizes[i] = remaining
		} else {
			size := (dimension * r) / total
			sizes[i] = size
			remaining -= size
		}
	}

	return sizes
}

// GetSlotNames returns all slot names in the layout
func (m *Manager) GetSlotNames() []string {
	return getSlotNamesRecursive(m.current.Slots)
}

func getSlotNamesRecursive(slots []Slot) []string {
	var names []string
	for _, slot := range slots {
		if len(slot.Children) > 0 {
			names = append(names, getSlotNamesRecursive(slot.Children)...)
		} else {
			names = append(names, slot.Name)
		}
	}
	return names
}
