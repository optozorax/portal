use portal::gui::scene::Scene;
use portal::gui::scene_serialized::normalize_pretty_output;
use portal::gui::scene_serialized::pretty_config;
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
        if content.trim().is_empty() {
            println!("Skipped empty: {}", path.display());
            continue;
        }
        if let Ok(scene) = ron::from_str::<Scene>(&content) {
            let ser = scene.to_serialized();
            let pretty = normalize_pretty_output(
                ron::ser::to_string_pretty(&ser, pretty_config())
                    .unwrap_or_else(|_| "<serialize error>".into()),
            );
            let new_path = path.clone();
            if let Err(err) = std::fs::write(&new_path, pretty) {
                eprintln!("Failed to write {}: {err}", new_path.display());
                continue;
            }
            println!("Converted: {} -> {}", path.display(), new_path.display());
        } else if let Ok(ser) = ron::from_str::<SerializedScene>(&content) {
            let pretty = normalize_pretty_output(
                ron::ser::to_string_pretty(&ser, pretty_config())
                    .unwrap_or_else(|_| "<serialize error>".into()),
            );
            if let Err(err) = std::fs::write(&path, pretty) {
                eprintln!("Failed to write {}: {err}", path.display());
                continue;
            }
            println!("Reformatted: {}", path.display());
        } else {
            eprintln!("Unrecognized format (skipped): {}", path.display());
        }
    }
}
