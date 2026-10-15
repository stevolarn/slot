use shelf_store::{Details, Platform};
use shelf_ui::{info_face, BACK_SCALE, INFO_H, INFO_W};

const SOR3: &str = "name=Streets of Rage 3\nyear=1994\ngenre=Beat 'em Up\ndeveloper=Sega AM7\n\
publisher=Sega of America\nplayers=2\ncoop=yes\nrating=4.20\nvotes=361\n\
overview=Snatch the Police Chief and replace him with a robot? Bomb city streets to splinters? \
Mr. X and the Syndicate surface once again to hatch the ultimate evil plot!\n";

fn shot(w: u32, h: u32, v: u8) -> shelf_ui::Shot {
    (vec![v; (w * h * 4) as usize], w, h)
}

#[test]
fn the_card_is_made_at_twice_its_size() {
    let d = Details::parse(SOR3);
    // INFO_SHOTS=title.jpg:play.jpg draws real screenshots.
    let shots: Vec<shelf_ui::Shot> = match std::env::var("INFO_SHOTS") {
        Ok(paths) => paths
            .split(':')
            .filter_map(|p| shelf_ui::info_shot(std::path::Path::new(p)))
            .collect(),
        Err(_) => vec![shot(320, 224, 40), shot(256, 224, 200)],
    };
    let face = info_face(&d, "Streets of Rage 3 (USA)", Platform::Md, &shots);
    assert_eq!((face.w, face.h), (INFO_W * BACK_SCALE, INFO_H * BACK_SCALE));
    assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
    // INFO_PNG=path writes the card out to look at.
    if let Ok(path) = std::env::var("INFO_PNG") {
        let file = std::fs::File::create(path).unwrap();
        let mut enc = png::Encoder::new(file, face.w, face.h);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header()
            .unwrap()
            .write_image_data(&face.rgba)
            .unwrap();
    }
}

#[test]
fn a_game_with_no_shots_or_details_still_gets_a_card() {
    let face = info_face(&Details::default(), "Some Game (USA)", Platform::Gb, &[]);
    assert_eq!((face.w, face.h), (INFO_W * BACK_SCALE, INFO_H * BACK_SCALE));
}
