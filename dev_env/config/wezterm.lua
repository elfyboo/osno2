local wezterm = require("wezterm")

return {
    font = wezterm.font("Consolas"),
    font_size = 12.0, -- point 3: overall size, tune to taste

    -- No tabs, no scrollbar, clean appliance look
    enable_tab_bar = false,
    scrollback_lines = 0,

    -- Fixed window geometry
    initial_cols = 220,
    initial_rows = 50,

    window_padding = {
        left = 0, right = 0, top = 0, bottom = 0,
    },

    colors = {
        background = "#050505",
    },
}
