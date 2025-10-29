use portal::gui::id_tools::stabilize_ids;
use portal::gui::scene::Scene;

fn main() {
    let scenes_dir = std::path::Path::new("scenes");
    let entries = match std::fs::read_dir(scenes_dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Failed to read scenes dir: {err}");
            std::process::exit(1);
        }
    };

    let pretty_cfg = ron::ser::PrettyConfig::default();

    // Ensure diffs root
    let diffs_root = std::path::Path::new("diffs");
    let _ = std::fs::create_dir_all(diffs_root);

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

        let scene: Scene = match ron::from_str(&content) {
            Ok(s) => s,
            Err(err) => {
                eprintln!(
                    "Unrecognized or invalid old Scene (skipped): {} ({err})",
                    path.display()
                );
                continue;
            }
        };

        // Roundtrip through new format and back
        let roundtripped = Scene::from_serialized(scene.to_serialized());

        // Stabilize ids in both scenes
        let s1 = stabilize_ids(scene, 64);
        let s2 = stabilize_ids(roundtripped, 64);

        // Pretty print
        let s1_str = ron::ser::to_string_pretty(&s1, pretty_cfg.clone())
            .unwrap_or_else(|_| "<serialize error>".into());
        let s2_str = ron::ser::to_string_pretty(&s2, pretty_cfg.clone())
            .unwrap_or_else(|_| "<serialize error>".into());

        // Prepare per-scene diff folder and files
        let scene_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let scene_dir = diffs_root.join(scene_name);
        if let Err(err) = std::fs::create_dir_all(&scene_dir) {
            eprintln!("Failed to create {}: {err}", scene_dir.display());
            continue;
        }
        let prev_path = scene_dir.join("previous.ron");
        let curr_path = scene_dir.join("current.ron");
        let diff_path = scene_dir.join("diff.patch");

        let _ = std::fs::write(&prev_path, &s1_str);
        let _ = std::fs::write(&curr_path, &s2_str);

        // Diff and write unified diff similar to git
        let diff = similar::TextDiff::from_lines(&s1_str, &s2_str);
        let changed_lines = diff
            .iter_all_changes()
            .filter(|c| !matches!(c.tag(), similar::ChangeTag::Equal))
            .count();

        // Build unified diff text
        let udiff = diff
            .unified_diff()
            .context_radius(3)
            .header("a/previous.ron", "b/current.ron")
            .to_string();
        let mut git_like = String::new();
        git_like.push_str("diff --git a/previous.ron b/current.ron\n");
        git_like.push_str(&udiff);
        let _ = std::fs::write(&diff_path, git_like);

        // Print summary
        if changed_lines == 0 {
            println!(
                "{}: no diff",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("<unknown>")
            );
        } else {
            println!(
                "{}: {} changed lines",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("<unknown>"),
                changed_lines
            );
        }
    }
}
