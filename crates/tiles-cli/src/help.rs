//! The `/help` text

pub(crate) fn show_help() {
    let help_groups = vec![
        (
            "Session",
            vec![
                (
                    "/reasoning <effort>",
                    "Sets the reasoning effort of current model (none, low, medium, high, xhigh)",
                ),
                ("/status", "Show the current session state"),
                ("/sessions", "List all available sessions"),
                (
                    "/resume <sessionId>",
                    "Load and resume a specific session (requires <sessionId>)",
                ),
            ],
        ),
        (
            "Sharing",
            vec![
                (
                    "/share",
                    "Create a shareable link for the current session (via ATProto)",
                ),
                (
                    "/share <sessionId>",
                    "Create a shareable link for a specific session (via ATProto)",
                ),
            ],
        ),
        (
            "Chat",
            vec![
                ("/help", "Show this help message"),
                ("/bye", "Exit the Chat"),
            ],
        ),
        (
            "Plugins",
            vec![
                ("/skills", "List all the available skills"),
                ("$<skill-name>", "Use the skill directly"),
            ],
        ),
    ];

    println!("Available Commands:");

    for (heading, commands) in help_groups {
        println!();
        println!("  {heading}");

        for (command, description) in commands {
            println!("    {command:<24}{description}");
        }
    }

    println!();
    println!("Documentation: https://tiles.run/book");
    println!("Report issues: https://github.com/tilesprivacy/tiles/issues");
    println!();
}
