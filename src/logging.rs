use crate::tailscale::localapi::LocalNodeInfo;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = match RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(5)
        .filename_prefix("tailscale-discloud")
        .filename_suffix(".log")
        .build("./logs")
    {
        Ok(appender) => appender,
        Err(e) => {
            tracing::error!("Failed to create log file appender: {}", e);
            panic!("Failed to create log file appender");
        }
    };

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
    let filter_str = if rust_log.is_empty() {
        "info,tailscale_tunnel_manager=info,api=info".to_string()
    } else if ["trace", "debug", "info", "warn", "error", "off"]
        .contains(&rust_log.to_lowercase().as_str())
    {
        format!(
            "info,tailscale_tunnel_manager={},api={}",
            rust_log, rust_log
        )
    } else {
        rust_log
    };
    let env_filter = EnvFilter::new(filter_str);

    let stdout_layer = fmt::layer()
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_target(true)
        .with_ansi(true);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    guard
}

fn clickable_terminal_link(url: &str) -> String {
    // For terminals that support OSC 8 hyperlinks (most Unix terminals), format the URL as a clickable link.
    format!("\x1b]8;;{url}\x1b\\{url}\x1b]8;;\x1b\\")
}

fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some(']') => {
                    for next_c in chars.by_ref() {
                        if next_c == '\\' {
                            break;
                        }
                    }
                }
                Some('[') => {
                    for next_c in chars.by_ref() {
                        if next_c == 'm' {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            len += 1;
        }
    }
    len
}

pub fn print_node_box(config: &LocalNodeInfo) {
    let mut lines = Vec::new();
    lines.push("You can connect to this node using:".to_string());

    if !config.dns_name.is_empty() {
        lines.push(format!(
            "  - DNSName:  {}",
            clickable_terminal_link(&format!("http://{}:3000/", config.dns_name))
        ));
    }
    if !config.magicdns_hostname.is_empty() {
        lines.push(format!(
            "  - MagicDNS: {}",
            clickable_terminal_link(&format!("http://{}:3000/", config.magicdns_hostname))
        ));
    }
    if !config.ipv4.is_empty() {
        lines.push(format!(
            "  - IPv4:     {}",
            clickable_terminal_link(&format!("http://{}:3000/", config.ipv4))
        ));
    }
    if !config.ipv6.is_empty() {
        lines.push(format!(
            "  - IPv6:     {}",
            clickable_terminal_link(&format!("http://[{}]:3000/", config.ipv6))
        ));
    }

    let max_len = lines.iter().map(|l| visible_len(l)).max().unwrap_or(0);
    let border_width = max_len + 4;

    let mut output = String::new();
    output.push_str("\n\x1b[96m┌");
    output.push_str(&"─".repeat(border_width - 2));
    output.push_str("┐\x1b[0m\n");

    for line in lines {
        let actual_visible = visible_len(&line);
        let padding = border_width - actual_visible - 4;
        output.push_str("\x1b[96m│\x1b[0m ");
        output.push_str(&line);
        output.push_str(&" ".repeat(padding));
        output.push_str(" \x1b[96m│\x1b[0m\n");
    }

    output.push_str("\x1b[96m└");
    output.push_str(&"─".repeat(border_width - 2));
    output.push_str("┘\x1b[0m\n");

    println!("{output}");
}
