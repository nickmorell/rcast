use egui::Context;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::audio_player::{AudioPlayer, PlaybackState};
use crate::commands::AppCommand;
use crate::components::add_podcast_modal::AddPodcastModal;
use crate::components::notes_panel::NotesPanel;
use crate::components::toast;
use crate::events::AppEvent;
use crate::pages::{home::HomePage, podcast_detail::PodcastDetailPage, settings::SettingsPage};
use crate::ports::{FilePicker, FolderPicker};
use crate::state::AppState;
use crate::types::Page;

pub struct RCast {
    pub cmd_tx: UnboundedSender<AppCommand>,
    pub event_rx: UnboundedReceiver<AppEvent>,
    pub state: AppState,
    pub current_page: Page,

    // ── Audio player (lives on App so the update loop can poll it) ────────────
    pub audio_player: AudioPlayer,

    // ── Global modal — top-level so it works on any page ─────────────────────
    pub add_podcast_modal: AddPodcastModal,

    // ── Notes panel — persists across pages, anchored to an episode ──────────
    pub notes_panel: NotesPanel,

    // ── Page structs (hold only transient local UI state) ─────────────────────
    pub home_page: HomePage,
    pub podcast_detail_page: PodcastDetailPage,
    pub settings_page: SettingsPage,
}

impl RCast {
    pub fn new(
        cmd_tx: UnboundedSender<AppCommand>,
        event_rx: UnboundedReceiver<AppEvent>,
        audio_player: AudioPlayer,
        folder_picker: Arc<dyn FolderPicker>,
        file_picker: Arc<dyn FilePicker>,
    ) -> Self {
        let _ = cmd_tx.send(AppCommand::NavigateTo(Page::Home));

        let mut settings_page = SettingsPage::default();
        settings_page.set_folder_picker(folder_picker);
        settings_page.set_file_picker(file_picker);

        Self {
            cmd_tx,
            event_rx,
            state: AppState::default(),
            current_page: Page::Home,
            audio_player,
            add_podcast_modal: AddPodcastModal::new(),
            notes_panel: NotesPanel::default(),
            home_page: HomePage::default(),
            podcast_detail_page: PodcastDetailPage::default(),
            settings_page,
        }
    }

    /// The only place in the application where `AppState` is mutated.
    /// All background tasks communicate back via `AppEvent`.
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            // ── Navigation ────────────────────────────────────────────────────
            AppEvent::NavigatedTo(page) => {
                self.current_page = page;
                // Clear stale detail data so the detail page shows a spinner
                // rather than the previous podcast's content.
                self.state.detail_podcast = None;
                self.state.detail_episodes.clear();
            }

            // ── Data ──────────────────────────────────────────────────────────
            AppEvent::PodcastsLoaded(podcasts) => {
                self.state.podcasts = podcasts;
            }
            AppEvent::PodcastAdded(podcast) => {
                self.state.podcasts.push(podcast);
                self.state
                    .toasts
                    .push(toast::ToastMessage::success("Podcast added!"));
            }
            AppEvent::PodcastRemoved(id) => {
                self.state.podcasts.retain(|p| p.id != id);
            }
            AppEvent::PodcastDetailLoaded { podcast, episodes } => {
                self.state.detail_podcast = Some(podcast);
                self.state.detail_episodes = episodes;
            }
            AppEvent::EpisodesUpdated {
                podcast_id,
                episodes,
            } => {
                if let Some(p) = self.state.podcasts.iter_mut().find(|p| p.id == podcast_id) {
                    p.episode_count = episodes.len() as i32;
                }
                if self
                    .state
                    .detail_podcast
                    .as_ref()
                    .map(|p| p.id == podcast_id)
                    .unwrap_or(false)
                {
                    self.state.detail_episodes = episodes;
                }
            }
            AppEvent::SyncStarted(podcast_id) => {
                self.state.syncing_podcast_ids.insert(podcast_id);
            }
            AppEvent::SyncCompleted(podcast_id) => {
                self.state.syncing_podcast_ids.remove(&podcast_id);
                // Refresh the podcast's last_synced_at in the list.
                // The orchestrator reloads podcasts after sync so this will
                // arrive shortly as a PodcastsLoaded event.
            }

            // ── Queue ─────────────────────────────────────────────────────────
            AppEvent::QueueUpdated(items) => {
                self.state.queue_display = items;
            }

            // ── Playback ──────────────────────────────────────────────────────
            AppEvent::PlaybackStarted {
                episode_id,
                podcast_id,
            } => {
                self.state.now_playing = Some(crate::state::NowPlaying {
                    episode_id,
                    podcast_id,
                });
            }
            AppEvent::PlaybackStopped => {
                self.state.now_playing = None;
            }

            // ── Settings ─────────────────────────────────────────────────────
            AppEvent::SettingsLoaded(settings) => {
                // Apply volume immediately to the already-running audio player.
                self.audio_player.set_volume(settings.default_volume);
                self.state.settings = settings.clone();
                self.settings_page.load(settings);
            }
            AppEvent::SettingsSaved => {
                // SettingsSaved is followed by SettingsLoaded, which re-applies
                // volume. Nothing else to do here.
            }

            AppEvent::OpmlImported {
                added,
                skipped,
                failed,
            } => {
                let msg = match (added, skipped, failed) {
                    (a, 0, 0) => format!("Imported {a} podcast{}", if a == 1 { "" } else { "s" }),
                    (a, s, 0) => format!("Imported {a}, skipped {s} already-subscribed"),
                    (a, 0, f) => format!("Imported {a}, {f} failed"),
                    (a, s, f) => format!("Imported {a}, skipped {s}, {f} failed"),
                };
                self.state.toasts.push(toast::ToastMessage::success(&msg));
            }
            AppEvent::OpmlExported { path } => {
                self.state
                    .toasts
                    .push(toast::ToastMessage::success(&format!("Exported to {path}")));
            }

            // ── Bookmarks ─────────────────────────────────────────────────────
            AppEvent::BookmarksLoaded {
                episode_bookmarks,
                podcast_bookmarks,
            } => {
                self.state.notes_episode_bookmarks = episode_bookmarks;
                self.state.notes_podcast_bookmarks = podcast_bookmarks;
            }
            AppEvent::BookmarkAdded(bookmark) => {
                if bookmark.episode_id.is_some() {
                    // Insert in sorted position: timed notes by position, then untimed
                    let pos = self.state.notes_episode_bookmarks.iter().position(|b| {
                        match (b.position_seconds, bookmark.position_seconds) {
                            (Some(a), Some(bv)) => a > bv,
                            (None, Some(_)) => true,
                            _ => false,
                        }
                    });
                    match pos {
                        Some(i) => self.state.notes_episode_bookmarks.insert(i, bookmark),
                        None => self.state.notes_episode_bookmarks.push(bookmark),
                    }
                } else {
                    self.state.notes_podcast_bookmarks.push(bookmark);
                }
            }
            AppEvent::BookmarkUpdated(updated) => {
                // The orchestrator sends back just id + new text — patch in place.
                for b in self
                    .state
                    .notes_episode_bookmarks
                    .iter_mut()
                    .chain(self.state.notes_podcast_bookmarks.iter_mut())
                {
                    if b.id == updated.id {
                        b.note_text = updated.note_text.clone();
                        break;
                    }
                }
            }
            AppEvent::BookmarkDeleted(id) => {
                self.state.notes_episode_bookmarks.retain(|b| b.id != id);
                self.state.notes_podcast_bookmarks.retain(|b| b.id != id);
            }

            // ── Cross-cutting ─────────────────────────────────────────────────
            AppEvent::Toast(msg) => {
                self.state.toasts.push(msg);
            }
            AppEvent::Error(msg) => {
                self.state.toasts.push(toast::ToastMessage::error(&msg));
            }
        }
    }

    /// Poll the audio player every frame to handle autoplay and track state.
    fn poll_audio(&mut self) {
        // Autoplay next in queue when the current track finishes.
        if self.audio_player.is_finished()
            && self.state.settings.auto_play_next
            && self.state.now_playing.is_some()
        {
            let current_id = self.audio_player.get_current_episode_id();
            // Guard against triggering multiple times for the same finish.
            if current_id != self.state.now_playing.as_ref().map(|_| -1) {
                let _ = self.cmd_tx.send(AppCommand::PlayNextInQueue);
            }
        }

        // Reset now_playing when audio stops without autoplay.
        if !self.audio_player.is_finished()
            && self.audio_player.get_state() == PlaybackState::Stopped
            && self.state.now_playing.is_some()
        {
            self.state.now_playing = None;
        }
    }
}

impl eframe::App for RCast {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // ── 1. Drain all pending background events ────────────────────────────
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event);
        }

        // ── 1b. Check if any page requested the Add Podcast modal ────────────
        if self.state.open_add_podcast_requested {
            self.add_podcast_modal.open();
            self.state.open_add_podcast_requested = false;
        }

        // ── 1c. Handle notes panel open requests (from detail page or media controls)
        if let Some((episode_id, podcast_id, title)) = self.state.notes_open_request.take() {
            let changed = self.notes_panel.open(episode_id, podcast_id, title);
            if changed {
                // Clear stale bookmarks immediately so the panel shows a clean state
                // while the fresh load is in flight.
                self.state.notes_episode_bookmarks.clear();
                self.state.notes_podcast_bookmarks.clear();
                let _ = self.cmd_tx.send(AppCommand::LoadBookmarks {
                    podcast_id,
                    episode_id,
                });
            }
        }

        // ── 2. Poll audio player for autoplay / stop detection ────────────────
        self.poll_audio();

        // Request repaints while audio is playing so the seek bar stays smooth.
        if self.audio_player.get_state() == PlaybackState::Playing {
            ctx.request_repaint();
        }

        // ── 3. Menu bar ───────────────────────────────────────────────────────
        if crate::components::menu::render(ctx, &self.state, &self.cmd_tx) {
            self.add_podcast_modal.open();
        }

        // ── 4. Notes panel (right SidePanel — must register before CentralPanel) ─
        {
            let now_playing_id = self.state.now_playing.as_ref().map(|np| np.episode_id);
            let current_pos = self.audio_player.get_position().as_secs_f64();
            self.notes_panel.render(
                ctx,
                &self.state.notes_episode_bookmarks,
                &self.state.notes_podcast_bookmarks,
                now_playing_id,
                current_pos,
                &self.cmd_tx,
            );
            if let Some(seek_to) = self.notes_panel.seek_request.take() {
                self.audio_player.seek(seek_to);
            }
        }

        // ── 5. Active page ────────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| match self.current_page.clone() {
            Page::Home => {
                self.home_page.render(ui, &mut self.state, &self.cmd_tx);
            }
            Page::PodcastDetail(_) => {
                self.podcast_detail_page
                    .render(ui, &mut self.state, &self.cmd_tx);
            }
            Page::Settings => {
                self.settings_page.render(ui, &mut self.state, &self.cmd_tx);
            }
        });

        // ── 6. Media controls (bottom panel) ─────────────────────────────────
        egui::TopBottomPanel::bottom("media_controls")
            .min_height(80.0)
            .show(ctx, |ui| {
                ui.add_space(5.0);

                // Determine the current podcast's image URL for the controls.
                let current_podcast_image = self.state.now_playing.as_ref().and_then(|np| {
                    self.state
                        .podcasts
                        .iter()
                        .find(|p| p.id == np.podcast_id)
                        .map(|p| p.image_url.clone())
                });

                // Find the current episode and its podcast title.
                let current_episode = self.state.now_playing.as_ref().and_then(|np| {
                    self.state
                        .detail_episodes
                        .iter()
                        .find(|e| e.id == np.episode_id)
                        .cloned()
                        .or_else(|| {
                            // If the detail page isn't loaded, we don't have the episode
                            // object handy. Audio still plays; the title is just blank.
                            None
                        })
                });

                let current_podcast_title = self.state.now_playing.as_ref().and_then(|np| {
                    self.state
                        .podcasts
                        .iter()
                        .find(|p| p.id == np.podcast_id)
                        .map(|p| p.title.clone())
                });

                use crate::components::media_controls::{MediaControls, MediaControlsAction};

                let mut volume = self.state.settings.default_volume;
                let action = MediaControls::render(
                    ui,
                    &self.audio_player,
                    &self.state.queue_display,
                    &self.state.image_cache,
                    &self.state.settings,
                    current_episode.as_ref(),
                    current_podcast_title.as_deref(),
                    current_podcast_image.as_deref(),
                    &mut volume,
                    &mut self.home_page.show_queue,
                    &mut self.home_page.show_speed_menu,
                    self.notes_panel.visible,
                );

                match action {
                    MediaControlsAction::ToggleNotes => {
                        if self.notes_panel.visible {
                            self.notes_panel.close();
                        } else if let Some(np) = &self.state.now_playing {
                            // Open panel for the currently playing episode.
                            let title = self
                                .state
                                .detail_episodes
                                .iter()
                                .find(|e| e.id == np.episode_id)
                                .map(|e| e.title.clone())
                                .unwrap_or_default();
                            self.state.notes_open_request =
                                Some((np.episode_id, np.podcast_id, title));
                        }
                    }
                    MediaControlsAction::PlayPause => match self.audio_player.get_state() {
                        PlaybackState::Playing => self.audio_player.pause(),
                        PlaybackState::Paused => self.audio_player.resume(),
                        _ => {}
                    },
                    MediaControlsAction::SkipBackward => {
                        self.audio_player
                            .skip_backward(self.state.settings.skip_backward_seconds);
                    }
                    MediaControlsAction::SkipForward => {
                        self.audio_player
                            .skip_forward(self.state.settings.skip_forward_seconds);
                    }
                    MediaControlsAction::Seek(pos) => {
                        self.audio_player.seek(pos);
                    }
                    MediaControlsAction::VolumeChanged(vol) => {
                        self.audio_player.set_volume(vol);
                        self.state.settings.default_volume = vol;
                        volume = vol;
                    }
                    MediaControlsAction::SetSpeed(speed) => {
                        self.audio_player.set_speed(speed);
                    }
                    MediaControlsAction::RemoveFromQueue(queue_id) => {
                        let _ = self.cmd_tx.send(AppCommand::RemoveFromQueue(queue_id));
                    }
                    MediaControlsAction::None => {}
                }

                ui.add_space(5.0);
            });

        // ── 7. Add Podcast modal (global — works on any page) ─────────────────
        if let Some(url) = self.add_podcast_modal.render(ctx) {
            let _ = self.cmd_tx.send(AppCommand::AddPodcast { feed_url: url });
        }

        // ── 8. Toast overlay ──────────────────────────────────────────────────
        toast::render(ctx, &mut self.state.toasts);
    }
}
