use crate::models::{AlertKind, IslandView, MusicVisualMode};
use crate::module::ModuleId;
use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderPolicy {
    pub main_visible: bool,
    pub pet_3d: bool,
    pub spectrum: bool,
    pub animations: bool,
}

pub fn render_policy(state: &AppState) -> RenderPolicy {
    if !state.is_visible {
        return RenderPolicy {
            main_visible: false,
            pet_3d: false,
            spectrum: false,
            animations: false,
        };
    }

    RenderPolicy {
        main_visible: true,
        pet_3d: active_slot_uses_pet(state),
        spectrum: active_slot_uses_spectrum(state),
        animations: true,
    }
}

fn active_slot_uses_pet(state: &AppState) -> bool {
    match state.mode {
        IslandView::Compact => compact_slot_uses_pet(state),
        IslandView::Wide => wide_slot_uses_pet(state),
        IslandView::Expand => expand_slot_uses_pet(state),
    }
}

fn compact_slot_uses_pet(state: &AppState) -> bool {
    if state.active_module == ModuleId::SYSTEM || state.active_module == ModuleId::MEDIA {
        true
    } else if state.active_module == ModuleId::ALERT {
        state.alert_kind == AlertKind::Charging
    } else {
        false
    }
}

fn wide_slot_uses_pet(state: &AppState) -> bool {
    if state.is_welcome {
        return true;
    }
    if state.show_source_selector {
        return false;
    }

    if state.active_module == ModuleId::SYSTEM {
        true
    } else if state.active_module == ModuleId::MEDIA {
        if state.media.is_playing {
            state.music_visual_mode == MusicVisualMode::PetPrimary
        } else {
            true
        }
    } else if state.active_module == ModuleId::ALERT {
        state.alert_kind == AlertKind::Charging
    } else {
        false
    }
}

fn expand_slot_uses_pet(state: &AppState) -> bool {
    if state.active_module == ModuleId::SYSTEM {
        true
    } else if state.active_module == ModuleId::MEDIA {
        if state.media.is_playing || state.show_source_selector {
            state.music_visual_mode == MusicVisualMode::PetPrimary
        } else {
            true
        }
    } else {
        false
    }
}

fn active_slot_uses_spectrum(state: &AppState) -> bool {
    if !state.media.is_playing {
        return false;
    }

    match state.mode {
        IslandView::Compact | IslandView::Expand => state.active_module == ModuleId::MEDIA,
        IslandView::Wide => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActiveModule;

    fn visible_state(mode: IslandView, module: impl Into<ModuleId>) -> AppState {
        AppState {
            is_visible: true,
            is_welcome: false,
            mode,
            active_module: module.into(),
            ..AppState::default()
        }
    }

    fn policy(pet_3d: bool, spectrum: bool) -> RenderPolicy {
        RenderPolicy {
            main_visible: true,
            pet_3d,
            spectrum,
            animations: true,
        }
    }

    #[test]
    fn hidden_stops_all_render_activity() {
        let mut state = visible_state(IslandView::Compact, ActiveModule::Media);
        state.media.is_playing = true;
        state.is_visible = false;

        assert_eq!(
            render_policy(&state),
            RenderPolicy {
                main_visible: false,
                pet_3d: false,
                spectrum: false,
                animations: false,
            }
        );
    }

    #[test]
    fn compact_media_uses_pet_and_spectrum_when_playing() {
        let mut state = visible_state(IslandView::Compact, ActiveModule::Media);
        state.media.is_playing = true;

        assert_eq!(render_policy(&state), policy(true, true));

        state.media.is_playing = false;
        assert_eq!(render_policy(&state), policy(true, false));
    }

    #[test]
    fn wide_media_uses_pet_only_in_pet_primary_mode() {
        let mut state = visible_state(IslandView::Wide, ActiveModule::Media);
        state.media.is_playing = true;
        assert_eq!(render_policy(&state), policy(false, false));

        state.music_visual_mode = MusicVisualMode::PetPrimary;
        assert_eq!(render_policy(&state), policy(true, false));
    }

    #[test]
    fn expand_media_uses_spectrum_and_pet_only_in_pet_primary_mode() {
        let mut state = visible_state(IslandView::Expand, ActiveModule::Media);
        state.media.is_playing = true;
        assert_eq!(render_policy(&state), policy(false, true));

        state.music_visual_mode = MusicVisualMode::PetPrimary;
        assert_eq!(render_policy(&state), policy(true, true));

        state.music_visual_mode = MusicVisualMode::ArtworkPrimary;
        state.media.is_playing = false;
        state.show_source_selector = true;
        assert_eq!(render_policy(&state), policy(false, false));

        state.music_visual_mode = MusicVisualMode::PetPrimary;
        assert_eq!(render_policy(&state), policy(true, false));
    }

    #[test]
    fn paused_media_falls_back_to_system_slot_pet() {
        for mode in [IslandView::Wide, IslandView::Expand] {
            let state = visible_state(mode, ActiveModule::Media);
            assert_eq!(render_policy(&state), policy(true, false));
        }
    }

    #[test]
    fn alert_and_prayer_slots_only_use_pet_when_rendered() {
        let mut compact_alert = visible_state(IslandView::Compact, ActiveModule::Alert);
        compact_alert.alert_kind = AlertKind::Charging;
        assert_eq!(render_policy(&compact_alert), policy(true, false));

        compact_alert.alert_kind = AlertKind::LowBattery;
        assert_eq!(render_policy(&compact_alert), policy(false, false));

        let mut wide_alert = visible_state(IslandView::Wide, ActiveModule::Alert);
        wide_alert.alert_kind = AlertKind::Charging;
        assert_eq!(render_policy(&wide_alert), policy(true, false));

        wide_alert.alert_kind = AlertKind::Prayer;
        assert_eq!(render_policy(&wide_alert), policy(false, false));

        for mode in [IslandView::Wide, IslandView::Expand] {
            let prayer = visible_state(mode, ActiveModule::Prayer);
            assert_eq!(render_policy(&prayer), policy(false, false));
        }

        let expand_alert = visible_state(IslandView::Expand, ActiveModule::Alert);
        assert_eq!(render_policy(&expand_alert), policy(false, false));
    }

    #[test]
    fn welcome_and_source_selector_override_wide_slots() {
        let mut state = visible_state(IslandView::Wide, ActiveModule::Prayer);
        state.is_welcome = true;
        assert_eq!(render_policy(&state), policy(true, false));

        state.is_welcome = false;
        state.active_module = ActiveModule::Media.into();
        state.media.is_playing = true;
        state.show_source_selector = true;
        assert_eq!(render_policy(&state), policy(false, false));
    }

    #[test]
    fn visible_tiers_produce_distinct_activity() {
        let mut compact = visible_state(IslandView::Compact, ActiveModule::Media);
        compact.media.is_playing = true;

        let mut wide = visible_state(IslandView::Wide, ActiveModule::Media);
        wide.media.is_playing = true;

        let mut expand = visible_state(IslandView::Expand, ActiveModule::Media);
        expand.media.is_playing = true;

        assert_eq!(render_policy(&compact), policy(true, true));
        assert_eq!(render_policy(&wide), policy(false, false));
        assert_eq!(render_policy(&expand), policy(false, true));
    }
}
