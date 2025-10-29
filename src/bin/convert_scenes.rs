use portal::gui::scene::Scene;
use portal::gui::scene_serialized::SerializedScene;

fn main() {
    let scenes_dir = std::path::Path::new("scenes");
    let entries = match std::fs::read_dir(scenes_dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Failed to read scenes dir: {err}");
            std::process::exit(1);
        }
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Skip {}: {err}", path.display());
                continue;
            }
        };
        // Always read old Scene and write new pretty format to .new.ron; do NOT overwrite old
        if let Ok(scene) = ron::from_str::<Scene>(&content) {
            let ser = scene.to_serialized();
            let pretty = ron::ser::to_string_pretty(&ser, ron::ser::PrettyConfig::default())
                .unwrap_or_else(|_| "<serialize error>".into());
            let new_path = path.clone(); //path.with_extension("new.ron");
            if let Err(err) = std::fs::write(&new_path, pretty) {
                eprintln!("Failed to write {}: {err}", new_path.display());
                continue;
            }
            println!("Converted: {} -> {}", path.display(), new_path.display());
        } else if ron::from_str::<SerializedScene>(&content).is_ok() {
            println!("Already new format: {}", path.display());
            // // Optionally write a pretty copy alongside
            // let ser: SerializedScene = ron::from_str(&content).unwrap();
            // let pretty = ron::ser::to_string_pretty(&ser, ron::ser::PrettyConfig::default()).unwrap_or_else(|_| "<serialize error>".into());
            // let pretty_path = path.with_extension("pretty.ron");
            // let _ = std::fs::write(&pretty_path, pretty);
        } else {
            eprintln!("Unrecognized format (skipped): {}", path.display());
        }
    }
}
