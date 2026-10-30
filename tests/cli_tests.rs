use clap::Parser;
use hyprsnip::cli::Cli;
use std::path::PathBuf;

#[test]
fn test_cli_no_args_defaults_to_saving_in_default_directory() {
    let cli = Cli::parse_from(["hyprsnip"]);
    assert_eq!(cli.output, None);
    assert!(!cli.clipboard);
    assert!(!cli.stdout);
    assert!(!cli.screen_on_empty);
    assert!(!cli.no_window);
    assert!(!cli.fullscreen);
    assert!(!cli.active);

    let plan = cli.plan_actions();
    assert_eq!(
        plan.save_path,
        Some(PathBuf::from("~/Pictures/Screenshots/"))
    );
    assert!(!plan.clipboard);
    assert!(!plan.stdout);
}

#[test]
fn test_cli_active_screen_flag() {
    let cli = Cli::parse_from(["hyprsnip", "-a"]);
    assert!(cli.active);
    let plan = cli.plan_actions();
    assert!(plan.clipboard);
    assert_eq!(plan.save_path, None);
}

#[test]
fn test_cli_fullscreen_chained_clipboard() {
    let cli = Cli::parse_from(["hyprsnip", "-fc"]);
    assert!(cli.fullscreen);
    assert!(cli.clipboard);
    let plan = cli.plan_actions();
    assert!(plan.clipboard);
    assert_eq!(plan.save_path, None);
}

#[test]
fn test_cli_output_directory() {
    let cli = Cli::parse_from(["hyprsnip", "-o", "~/Screenshots"]);
    assert_eq!(cli.output, Some(PathBuf::from("~/Screenshots")));
    assert!(!cli.clipboard);
    assert!(!cli.stdout);

    let plan = cli.plan_actions();
    assert_eq!(plan.save_path, Some(PathBuf::from("~/Screenshots")));
    assert!(!plan.clipboard);
    assert!(!plan.stdout);
}

#[test]
fn test_cli_output_specific_file() {
    let cli = Cli::parse_from(["hyprsnip", "--output", "~/Screenshots/my_screenshot.png"]);
    assert_eq!(
        cli.output,
        Some(PathBuf::from("~/Screenshots/my_screenshot.png"))
    );

    let plan = cli.plan_actions();
    assert_eq!(
        plan.save_path,
        Some(PathBuf::from("~/Screenshots/my_screenshot.png"))
    );
    assert!(!plan.clipboard);
    assert!(!plan.stdout);
}

#[test]
fn test_cli_clipboard_only() {
    let cli = Cli::parse_from(["hyprsnip", "-c"]);
    assert!(cli.clipboard);
    assert!(!cli.stdout);
    assert_eq!(cli.output, None);

    let plan = cli.plan_actions();

    assert_eq!(plan.save_path, None);
    assert!(plan.clipboard);
    assert!(!plan.stdout);
}

#[test]
fn test_cli_stdout_only() {
    let cli = Cli::parse_from(["hyprsnip", "--stdout"]);
    assert!(cli.stdout);
    assert!(!cli.clipboard);
    assert_eq!(cli.output, None);

    let plan = cli.plan_actions();

    assert_eq!(plan.save_path, None);
    assert!(!plan.clipboard);
    assert!(plan.stdout);
}

#[test]
fn test_cli_combined_output_and_clipboard() {
    let cli = Cli::parse_from(["hyprsnip", "-o", "~/Screenshots/shot.png", "-c"]);
    let plan = cli.plan_actions();

    assert_eq!(
        plan.save_path,
        Some(PathBuf::from("~/Screenshots/shot.png"))
    );
    assert!(plan.clipboard);
    assert!(!plan.stdout);
}

#[test]
fn test_cli_combined_all_three() {
    let cli = Cli::parse_from(["hyprsnip", "-o", "~/Screenshots", "-c", "--stdout"]);
    let plan = cli.plan_actions();

    assert_eq!(plan.save_path, Some(PathBuf::from("~/Screenshots")));
    assert!(plan.clipboard);
    assert!(plan.stdout);
}

#[test]
fn test_cli_screen_on_empty_and_no_window_flags() {
    let cli = Cli::parse_from(["hyprsnip", "-s", "--no-window"]);
    assert!(cli.screen_on_empty);
    assert!(cli.no_window);
}

#[test]
fn test_cli_existing_customization_flags_preserved() {
    let cli = Cli::parse_from([
        "hyprsnip",
        "--stroke-color",
        "#ff0000ff",
        "--dim-color",
        "#11223344",
        "--inside-color",
        "#55667788",
        "--no-rounding",
    ]);
    assert_eq!(cli.stroke_color, "#ff0000ff");
    assert_eq!(cli.dim_color, "#11223344");
    assert_eq!(cli.inside_color, "#55667788");
    assert!(cli.no_rounding);
}
