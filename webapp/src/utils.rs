/// Tailwind classes for a line, using TfL's official line colours.
pub struct LineTheme {
    /// Solid header background and contrasting text
    pub header: &'static str,
    /// Light background tint
    pub light: &'static str,
    /// Border tint
    pub border: &'static str,
}

const fn theme(header: &'static str, light: &'static str, border: &'static str) -> LineTheme {
    LineTheme {
        header,
        light,
        border,
    }
}

// Class strings must stay literal so Tailwind can find them when scanning sources.
pub fn line_theme(line_name: &str) -> LineTheme {
    match line_name.trim_end_matches(" line") {
        "Bakerloo" => theme(
            "bg-[#B36305] text-white",
            "bg-[#B36305]/10",
            "border-[#B36305]/30",
        ),
        "Central" => theme(
            "bg-[#E32017] text-white",
            "bg-[#E32017]/10",
            "border-[#E32017]/30",
        ),
        "Circle" => theme(
            "bg-[#FFD300] text-black",
            "bg-[#FFD300]/10",
            "border-[#FFD300]/40",
        ),
        "District" => theme(
            "bg-[#00782A] text-white",
            "bg-[#00782A]/10",
            "border-[#00782A]/30",
        ),
        "Hammersmith & City" => theme(
            "bg-[#F3A9BB] text-black",
            "bg-[#F3A9BB]/15",
            "border-[#F3A9BB]/50",
        ),
        "Jubilee" => theme(
            "bg-[#A0A5A9] text-black",
            "bg-[#A0A5A9]/15",
            "border-[#A0A5A9]/50",
        ),
        "Metropolitan" => theme(
            "bg-[#9B0056] text-white",
            "bg-[#9B0056]/10",
            "border-[#9B0056]/30",
        ),
        "Northern" => theme("bg-black text-white", "bg-gray-50", "border-gray-300"),
        "Piccadilly" => theme(
            "bg-[#003688] text-white",
            "bg-[#003688]/10",
            "border-[#003688]/30",
        ),
        "Victoria" => theme(
            "bg-[#0098D4] text-white",
            "bg-[#0098D4]/10",
            "border-[#0098D4]/30",
        ),
        "Waterloo & City" => theme(
            "bg-[#95CDBA] text-black",
            "bg-[#95CDBA]/15",
            "border-[#95CDBA]/50",
        ),
        "DLR" => theme(
            "bg-[#00A4A7] text-white",
            "bg-[#00A4A7]/10",
            "border-[#00A4A7]/30",
        ),
        "Elizabeth" => theme(
            "bg-[#6950A1] text-white",
            "bg-[#6950A1]/10",
            "border-[#6950A1]/30",
        ),
        // London Overground lines (named in 2024)
        "Liberty" => theme(
            "bg-[#61686B] text-white",
            "bg-[#61686B]/10",
            "border-[#61686B]/30",
        ),
        "Lioness" => theme(
            "bg-[#F1B41C] text-black",
            "bg-[#F1B41C]/15",
            "border-[#F1B41C]/50",
        ),
        "Mildmay" => theme(
            "bg-[#437EC1] text-white",
            "bg-[#437EC1]/10",
            "border-[#437EC1]/30",
        ),
        "Suffragette" => theme(
            "bg-[#39B97A] text-white",
            "bg-[#39B97A]/10",
            "border-[#39B97A]/30",
        ),
        "Weaver" => theme(
            "bg-[#972861] text-white",
            "bg-[#972861]/10",
            "border-[#972861]/30",
        ),
        "Windrush" => theme(
            "bg-[#EF4D5E] text-white",
            "bg-[#EF4D5E]/10",
            "border-[#EF4D5E]/30",
        ),
        "London Overground" => theme(
            "bg-[#EE7C0E] text-white",
            "bg-[#EE7C0E]/10",
            "border-[#EE7C0E]/30",
        ),
        _ => theme("bg-gray-700 text-white", "bg-gray-100", "border-gray-200"),
    }
}

/// Classes for the "track" button, keyed by the API's `modeName`.
pub fn mode_button_classes(mode_name: &str) -> &'static str {
    match mode_name {
        "tube" => "bg-blue-100 text-blue-800 p-1 rounded hover:opacity-80",
        "dlr" => "bg-teal-100 text-teal-800 p-1 rounded hover:opacity-80",
        "overground" => "bg-orange-100 text-orange-800 p-1 rounded hover:opacity-80",
        "elizabeth-line" => "bg-purple-100 text-purple-800 p-1 rounded hover:opacity-80",
        _ => "bg-gray-100 text-gray-800 p-1 rounded hover:opacity-80",
    }
}

pub fn format_arrival_time(seconds: i32) -> String {
    if seconds < 60 {
        "Due".to_string()
    } else {
        format!("{} min", seconds / 60)
    }
}

pub fn get_status_color(status: &str) -> &'static str {
    match status {
        "Arrived" => "bg-green-100 text-green-800",
        "Arriving" => "bg-yellow-100 text-yellow-800",
        "Approaching" => "bg-blue-100 text-blue-800",
        "Not Found" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overground_lines_have_their_own_colours() {
        for line in [
            "Liberty",
            "Lioness",
            "Mildmay",
            "Suffragette",
            "Weaver",
            "Windrush",
        ] {
            assert_ne!(
                line_theme(line).header,
                line_theme("Unknown").header,
                "{line}"
            );
        }
    }

    #[test]
    fn elizabeth_line_matches_with_or_without_suffix() {
        assert_eq!(
            line_theme("Elizabeth line").header,
            line_theme("Elizabeth").header
        );
    }

    #[test]
    fn unknown_lines_fall_back_to_grey_not_northern() {
        assert_ne!(line_theme("Unknown").header, line_theme("Northern").header);
    }

    #[test]
    fn mode_colours_match_api_mode_names() {
        assert!(mode_button_classes("tube").contains("blue"));
        assert!(mode_button_classes("elizabeth-line").contains("purple"));
    }

    #[test]
    fn arrival_time_formatting() {
        assert_eq!(format_arrival_time(59), "Due");
        assert_eq!(format_arrival_time(60), "1 min");
        assert_eq!(format_arrival_time(179), "2 min");
    }
}
