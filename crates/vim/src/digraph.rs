use std::sync::Arc;

use collections::HashMap;
use gpui::AppContext;
use lazy_static::lazy_static;
use settings::Settings;
use ui::WindowContext;

use crate::{Vim, VimSettings};

mod default;

lazy_static! {
    static ref DEFAULT_DIGRAPHS_MAP: HashMap<String, Arc<str>> = {
        let mut map = HashMap::default();
        for &(a, b, c) in default::DEFAULT_DIGRAPHS {
            let key = format!("{a}{b}");
            let value = char::from_u32(c).unwrap().to_string().into();
            map.insert(key, value);
        }
        map
    };
}

fn lookup_digraph(a: char, b: char, cx: &AppContext) -> Arc<str> {
    let custom_digraphs = &VimSettings::get_global(cx).custom_digraphs;
    let input = format!("{a}{b}");
    let reversed = format!("{b}{a}");

    custom_digraphs
        .get(&input)
        .or_else(|| DEFAULT_DIGRAPHS_MAP.get(&input))
        .or_else(|| custom_digraphs.get(&reversed))
        .or_else(|| DEFAULT_DIGRAPHS_MAP.get(&reversed))
        .cloned()
        .unwrap_or_else(|| b.to_string().into())
}

pub fn insert_digraph(first_char: char, second_char: char, cx: &mut WindowContext) {
    let text = lookup_digraph(first_char, second_char, &cx);

    Vim::update(cx, |vim, cx| vim.pop_operator(cx));
    if Vim::read(cx).state().editor_input_enabled() {
        Vim::update(cx, |vim, cx| {
            vim.update_active_editor(cx, |_, editor, cx| editor.insert(&text, cx));
        });
    } else {
        Vim::active_editor_input_ignored(text, cx);
    }
}

#[cfg(test)]
mod test {
    use collections::HashMap;
    use settings::SettingsStore;

    use crate::{
        state::Mode,
        test::{NeovimBackedTestContext, VimTestContext},
        VimSettings,
    };

    #[gpui::test]
    async fn test_digraph_insert_mode(cx: &mut gpui::TestAppContext) {
        let mut cx: NeovimBackedTestContext = NeovimBackedTestContext::new(cx).await;

        cx.set_shared_state("Hellˇo").await;
        cx.simulate_shared_keystrokes("a ctrl-k o : escape").await;
        cx.shared_state().await.assert_eq("Helloˇö");

        cx.set_shared_state("Hellˇo").await;
        cx.simulate_shared_keystrokes("a ctrl-k : o escape").await;
        cx.shared_state().await.assert_eq("Helloˇö");

        cx.set_shared_state("Hellˇo").await;
        cx.simulate_shared_keystrokes("i ctrl-k o : escape").await;
        cx.shared_state().await.assert_eq("Hellˇöo");
    }

    #[gpui::test]
    async fn test_digraph_insert_multicursor(cx: &mut gpui::TestAppContext) {
        let mut cx: VimTestContext = VimTestContext::new(cx, true).await;

        cx.set_state("Hellˇo wˇorld", Mode::Normal);
        cx.simulate_keystrokes("a ctrl-k o : escape");
        cx.assert_state("Helloˇö woˇörld", Mode::Normal);
    }

    #[gpui::test]
    async fn test_digraph_replace(cx: &mut gpui::TestAppContext) {
        let mut cx: NeovimBackedTestContext = NeovimBackedTestContext::new(cx).await;

        cx.set_shared_state("Hellˇo").await;
        cx.simulate_shared_keystrokes("r ctrl-k o :").await;
        cx.shared_state().await.assert_eq("Hellˇö");
    }

    #[gpui::test]
    async fn test_digraph_find(cx: &mut gpui::TestAppContext) {
        let mut cx: NeovimBackedTestContext = NeovimBackedTestContext::new(cx).await;

        cx.set_shared_state("ˇHellö world").await;
        cx.simulate_shared_keystrokes("f ctrl-k o :").await;
        cx.shared_state().await.assert_eq("Hellˇö world");

        cx.set_shared_state("ˇHellö world").await;
        cx.simulate_shared_keystrokes("t ctrl-k o :").await;
        cx.shared_state().await.assert_eq("Helˇlö world");
    }

    #[gpui::test]
    async fn test_digraph_replace_mode(cx: &mut gpui::TestAppContext) {
        let mut cx: NeovimBackedTestContext = NeovimBackedTestContext::new(cx).await;

        cx.set_shared_state("ˇHello").await;
        cx.simulate_shared_keystrokes(
            "shift-r ctrl-k a ' ctrl-k e ` ctrl-k i : ctrl-k o ~ ctrl-k u - escape",
        )
        .await;
        cx.shared_state().await.assert_eq("áèïõˇū");
    }

    #[gpui::test]
    async fn test_digraph_custom(cx: &mut gpui::TestAppContext) {
        let mut cx: VimTestContext = VimTestContext::new(cx, true).await;

        cx.update_global(|store: &mut SettingsStore, cx| {
            store.update_user_settings::<VimSettings>(cx, |s| {
                let mut custom_digraphs = HashMap::default();
                custom_digraphs.insert("|-".into(), "⊢".into());
                custom_digraphs.insert(":)".into(), "👨‍💻".into());
                s.custom_digraphs = Some(custom_digraphs);
            });
        });

        cx.set_state("ˇ", Mode::Normal);
        cx.simulate_keystrokes("a ctrl-k | - escape");
        cx.assert_state("ˇ⊢", Mode::Normal);

        // Test support for multi-codepoint mappings
        cx.set_state("ˇ", Mode::Normal);
        cx.simulate_keystrokes("a ctrl-k : ) escape");
        cx.assert_state("ˇ👨‍💻", Mode::Normal);
    }
}
