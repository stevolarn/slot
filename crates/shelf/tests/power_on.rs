use shelf::input::evdev::held_buttons;
use shelf::power_on::{clear_tries, skips_shelf, tries_path};
use shelf_input::Btn;

#[test]
fn select_or_menu_held_sends_the_brick_to_nextui() {
    assert!(skips_shelf(&[Btn::A, Btn::Select]));
    assert!(skips_shelf(&[Btn::Menu]));
    assert!(!skips_shelf(&[Btn::A, Btn::Start]));
    assert!(!skips_shelf(&[]));
}

#[test]
fn clearing_the_tries_removes_the_count_and_is_fine_with_none() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    clear_tries(d.path());
    std::fs::write(tries_path(d.path()), "1\n").unwrap();
    clear_tries(d.path());
    assert!(!tries_path(d.path()).exists());
}

#[test]
fn no_input_devices_means_no_buttons_held() {
    let d = tempfile::tempdir().unwrap();
    assert!(held_buttons(d.path(), d.path()).is_empty());
}
