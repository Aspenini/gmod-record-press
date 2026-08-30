use crate::model::AlbumProject;
use crate::slug::lua_escape;

pub fn render_autorun(project: &AlbumProject, tracks: &[(String, String)]) -> String {
    let id = project.vinyl_id.trim();
    let hook = format!("recordplayer-{id}");
    let mut out = String::new();

    out.push_str(&format!(
        "hook.Add(\"RecordPlayer:CollectVinyls\", {}, function()\n",
        lua_escape(&hook)
    ));
    out.push_str(&format!(
        "    RecordPlayer.RegisterVinyl({}, {{\n",
        lua_escape(id)
    ));
    out.push_str(&format!(
        "        name = {},\n",
        lua_escape(project.album.trim())
    ));
    out.push_str(&format!(
        "        artist = {},\n",
        lua_escape(project.artist.trim())
    ));
    out.push_str("        tracks = {\n");
    for (index, (name, sound)) in tracks.iter().enumerate() {
        let comma = if index + 1 == tracks.len() { "" } else { "," };
        out.push_str(&format!(
            "            {{name = {}, sound = {}}}{comma}\n",
            lua_escape(name),
            lua_escape(sound)
        ));
    }
    out.push_str("        },\n");
    out.push_str(&format!(
        "        cover = {},\n",
        lua_escape(&format!("recordplayer/{id}/cover.png"))
    ));
    out.push_str(&format!(
        "        caseFrontMaterial = {},\n",
        lua_escape(&format!("recordplayer/{id}/case_front"))
    ));
    out.push_str(&format!(
        "        caseBackMaterial = {},\n",
        lua_escape(&format!("recordplayer/{id}/case_back"))
    ));
    out.push_str(&format!(
        "        vinylMaterial = {}\n",
        lua_escape(&format!("recordplayer/{id}/vinyl"))
    ));
    out.push_str("    })\n");
    out.push_str("end)\n");
    out
}

pub fn case_vmt(id: &str, name: &str) -> String {
    format!(
        "\"VertexLitGeneric\"\n\
{{\n\
\t\"$basetexture\" \"recordplayer/{id}/{name}\"\n\
\t\"$basetexturetransform\" \"center 0 0 scale 2 2 rotate 0 translate 0 0\"\n\
}}\n"
    )
}

pub fn vinyl_vmt(id: &str) -> String {
    format!(
        "\"VertexLitGeneric\"\n\
{{\n\
    \"$basetexture\"   \"recordplayer/{id}/vinyl\"\n\
    \"$bumpmap\"       \"models/textures/vinyl_n\"\n\
    \"$normalmapalphaenvmapmask\" \"1\"\n\
    \"$envmaptint\" \"[ 0.1 0.1 0.2 ]\"\n\
\n\
    \"$phong\"     \"1\"\n\
    \"$phongexponent\"  \"30\"\n\
    \"$phongboost\"   \"1\"\n\
    \"$phongfresnelranges\"   \"[1 9 3]\"\n\
    \"$phongtint\"                   \"[0.7 0.9 1]\"\n\
    \"$phongalbedotint\" \"1\"\n\
    \"$halflambert\" \"1\"\n\
\n\
    \"$rimlight\" \"1\"\n\
    \"$rimlightexponent\" \"26\"\n\
    \"$rimlightboost\" \"0.2\"\n\
    \"$selfillum\" \"1\"\n\
}}\n"
    )
}

pub fn addon_json(title: &str) -> serde_json::Value {
    serde_json::json!({
        "title": title,
        "type": "entity",
        "tags": ["fun", "realism", "roleplay"],
        "ignore": []
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AlbumProject, Track};

    #[test]
    fn lua_matches_register_shape() {
        let project = AlbumProject {
            artist: "Black Sabbath".into(),
            album: "Paranoid".into(),
            vinyl_id: "paranoid".into(),
            addon_title: String::new(),
            cover_path: None,
            back_cover_path: None,
            label_path: None,
            vinyl_color: "#141414".into(),
            vinyl_resolution: 2048,
            tracks: vec![Track {
                name: "War Pigs / Luke's Wall".into(),
                path: "x.mp3".into(),
            }],
        };
        let lua = render_autorun(
            &project,
            &[(
                "War Pigs / Luke's Wall".into(),
                "recordplayer/paranoid/war_pigs.mp3".into(),
            )],
        );
        assert!(lua.contains("RecordPlayer:CollectVinyls"));
        assert!(lua.contains("RecordPlayer.RegisterVinyl(\"paranoid\""));
        assert!(lua.contains("caseFrontMaterial = \"recordplayer/paranoid/case_front\""));
        assert!(lua.contains("sound = \"recordplayer/paranoid/war_pigs.mp3\""));
    }
}
