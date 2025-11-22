use caramel::ns::format::{nation_link, region_link, prettify_name};

pub fn display_nation(name: &str, bold: bool) -> String {
    let display = prettify_name(name);
    let fmt = if bold { format!("**{}**", display) } else { display };
    format!("[{fmt}]({})", nation_link(name))
}

pub fn display_region(name: &str, bold: bool) -> String {
    let display = prettify_name(name);
    let fmt = if bold { format!("**{}**", display) } else { display };
    format!("[{fmt}]({})", region_link(name))
}