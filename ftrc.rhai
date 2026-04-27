#{
    core: #{
	mod_key: "Super",
	terminal: "alacritty",
	launcher: "rofi -show drun",
	focus_follows_mouse: true,
	auto_raise: false
    },

    keybinds: #{
	launch_terminal: "Mod+Enter",
	launch_launcher: "Mod+D",
	close_window: "Mod+Shift+Q",
	fullscreen: "Mod+F",
	toggle_floating: "Mod+Shift+Space",
	reload_config: "Mod+Shift+R"
    },

    workspaces: #{
	count: 10,
	names: [
	    "1", "2", "3", "4", "5",
	    "6", "7", "8", "9", "10"
	]
    },

    window_rules: [
	#{
	    class: "firefox",
	    workspace: 2,
	    floating: false
	},
	#{
	    class: "pavucontrol",
	    floating: true
	},
	#{
	    role: "dialog",
	    floating: true,
	    always_on_top: true
	}
    ],

    layout: #{
	tiling: true,
	default_split: "horizontal",
	gap_inner: 3,
	gap_outer: 6,
	border_width: 2,
	smart_gaps: true,
	master_ratio: 0.6
    },

    input: #{
	keyboard_layout: "us",
	repeat_delay: 250,
	repeat_rate: 35,
	mouse_accel: 0.0,
	touchpad: #{
	    tap_to_click: true,
	    natural_scroll: true
	}
    },

    monitor: #{
	primary: "DP-1",
	scaling: 1.0,
	workspaces_per_monitor: true
    },

    system: #{
	autostart: [
	    "nm-applet",
	    "picom",
	    "dunst"
	],
	lock_command: "swaylock",
	screenshot: "grim + slurp"
    }
}
