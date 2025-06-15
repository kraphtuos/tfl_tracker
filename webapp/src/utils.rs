fn get_line_key(line_name: &str) -> &'static str {
    if line_name.contains("Bakerloo") {
        "Bakerloo"
    } else if line_name.contains("Central") {
        "Central"
    } else if line_name.contains("Circle") {
        "Circle"
    } else if line_name.contains("District") {
        "District"
    } else if line_name.contains("Hammersmith") {
        "Hammersmith"
    } else if line_name.contains("Jubilee") {
        "Jubilee"
    } else if line_name.contains("Metropolitan") {
        "Metropolitan"
    } else if line_name.contains("Northern") {
        "Northern"
    } else if line_name.contains("Piccadilly") {
        "Piccadilly"
    } else if line_name.contains("Victoria") {
        "Victoria"
    } else if line_name.contains("Waterloo") {
        "Waterloo"
    } else if line_name.contains("DLR") {
        "DLR"
    } else if line_name.contains("Elizabeth") {
        "Elizabeth"
    } else if line_name.contains("Overground") {
        "Overground"
    } else {
        "Northern"
    } // fallback
}

fn get_line_theme(key: &str) -> (&'static str, &'static str, &'static str) {
    match key {
        "Bakerloo" => (
            "bg-orange-700 text-white",
            "bg-orange-50",
            "border-orange-200",
        ),
        "Central" => ("bg-red-600 text-white", "bg-red-50", "border-red-200"),
        "Circle" => (
            "bg-yellow-400 text-black",
            "bg-yellow-50",
            "border-yellow-200",
        ),
        "District" => ("bg-green-600 text-white", "bg-green-50", "border-green-200"),
        "Hammersmith" => ("bg-pink-400 text-white", "bg-pink-50", "border-pink-200"),
        "Jubilee" => ("bg-gray-500 text-white", "bg-gray-50", "border-gray-200"),
        "Metropolitan" => (
            "bg-purple-700 text-white",
            "bg-purple-50",
            "border-purple-200",
        ),
        "Northern" => ("bg-black text-white", "bg-gray-50", "border-gray-200"),
        "Piccadilly" => ("bg-blue-800 text-white", "bg-blue-50", "border-blue-200"),
        "Victoria" => ("bg-blue-400 text-white", "bg-blue-50", "border-blue-200"),
        "Waterloo" => ("bg-cyan-500 text-white", "bg-cyan-50", "border-cyan-200"),
        "DLR" => ("bg-teal-500 text-white", "bg-teal-50", "border-teal-200"),
        "Elizabeth" => (
            "bg-purple-500 text-white",
            "bg-purple-50",
            "border-purple-200",
        ),
        "Overground" => (
            "bg-orange-500 text-white",
            "bg-orange-50",
            "border-orange-200",
        ),
        _ => ("bg-gray-700 text-white", "bg-gray-100", "border-gray-200"), // fallback
    }
}

pub fn get_line_colors(line_name: &str) -> String {
    let key = get_line_key(line_name);
    let (color, _, _) = get_line_theme(key);
    format!("{} font-bold py-2 px-3", color)
}

pub fn get_line_background_colors(line_name: &str) -> String {
    let key = get_line_key(line_name);
    let (_, bg, _) = get_line_theme(key);
    format!("{} font-medium py-2 px-3", bg)
}

pub fn get_line_theme_colors(line_name: &str) -> (String, String) {
    let key = get_line_key(line_name);
    let (_, bg, border) = get_line_theme(key);
    (bg.to_string(), border.to_string())
}

pub fn get_mode_colors(mode_name: &str) -> String {
    let color = match mode_name {
        "Underground" => "bg-blue-100 text-blue-800",
        "DLR" => "bg-teal-100 text-teal-800",
        "Overground" => "bg-orange-100 text-orange-800",
        "Elizabeth" => "bg-purple-100 text-purple-800",
        _ => "bg-gray-100 text-gray-800",
    };
    format!("{} p-1 rounded hover:opacity-80", color)
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
