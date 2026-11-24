//! Album/artist grouping and the asynchronous album-cover grid.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use eframe::egui;
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, TextureHandle,
    TextureOptions, Ui, Vec2,
};

use super::RPlayerApp;
use crate::cover;
use crate::model::Track;
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets;

pub(super) fn grid_columns(available: f32) -> usize {
    const GAP: f32 = 24.0;
    const MIN_CARD: f32 = 188.0;
    (((available + GAP) / (MIN_CARD + GAP)).floor() as usize).clamp(3, 5)
}

#[derive(Debug, Clone)]
pub(super) struct AlbumGroup {
    pub key: String,
    pub title: String,
    pub artist: String,
    pub year: Option<u32>,
    pub ids: Vec<u64>,
    pub duration: f64,
    pub first: Track,
}

#[derive(Debug, Clone)]
pub(super) struct ArtistGroup {
    pub name: String,
    pub ids: Vec<u64>,
    pub album_count: usize,
    pub first: Track,
}

struct ThumbRequest {
    key: String,
    track: Track,
}

struct ThumbReply {
    key: String,
    image: Option<egui::ColorImage>,
}

pub(super) struct BrowseState {
    textures: HashMap<String, TextureHandle>,
    requested: HashSet<String>,
    tx: Sender<ThumbRequest>,
    rx: Receiver<ThumbReply>,
}

impl BrowseState {
    pub fn new() -> Self {
        let (tx, request_rx) = mpsc::channel::<ThumbRequest>();
        let (reply_tx, rx) = mpsc::channel::<ThumbReply>();
        std::thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                let image = load_thumbnail(&request.track);
                if reply_tx
                    .send(ThumbReply {
                        key: request.key,
                        image,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            textures: HashMap::new(),
            requested: HashSet::new(),
            tx,
            rx,
        }
    }

    fn ensure(&mut self, key: &str, track: &Track) {
        if self.textures.contains_key(key) || self.requested.contains(key) {
            return;
        }
        self.requested.insert(key.to_string());
        let _ = self.tx.send(ThumbRequest {
            key: key.to_string(),
            track: track.clone(),
        });
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        loop {
            let reply = match self.rx.try_recv() {
                Ok(reply) => reply,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            };
            let Some(image) = reply.image else {
                continue;
            };
            let texture = ctx.load_texture(
                format!("rplayer-album-thumb:{}", reply.key),
                image,
                TextureOptions::LINEAR,
            );
            self.textures.insert(reply.key, texture);
        }
    }

    fn texture(&self, key: &str) -> Option<TextureHandle> {
        self.textures.get(key).cloned()
    }
}

fn load_thumbnail(track: &Track) -> Option<egui::ColorImage> {
    let bytes = cover::load_local(track)
        .map(|cover| cover.bytes)
        .or_else(|| cover::read_cache(track))?;
    let image = image::load_from_memory(&bytes).ok()?;
    let thumb = image.thumbnail(256, 256);
    let rgba = thumb.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

impl RPlayerApp {
    pub(super) fn album_groups(&self, ids: &[u64]) -> Vec<AlbumGroup> {
        let mut groups: BTreeMap<String, AlbumGroup> = BTreeMap::new();
        for id in ids {
            let Some(track) = self.library.track(*id) else {
                continue;
            };
            let key = album_key(track);
            let entry = groups.entry(key.clone()).or_insert_with(|| AlbumGroup {
                key,
                title: display_album(track),
                artist: display_artist(track),
                year: track.year,
                ids: Vec::new(),
                duration: 0.0,
                first: track.clone(),
            });
            if entry.year.is_none() {
                entry.year = track.year;
            }
            entry.ids.push(track.id);
            entry.duration += track.duration;
        }
        let mut result: Vec<_> = groups.into_values().collect();
        result.sort_by_key(|group| group.title.to_lowercase());
        result
    }

    pub(super) fn artist_groups(&self, ids: &[u64]) -> Vec<ArtistGroup> {
        let mut groups: BTreeMap<String, ArtistGroup> = BTreeMap::new();
        let mut albums: HashMap<String, HashSet<String>> = HashMap::new();
        for id in ids {
            let Some(track) = self.library.track(*id) else {
                continue;
            };
            let name = display_artist(track);
            let key = name.to_lowercase();
            let entry = groups.entry(key.clone()).or_insert_with(|| ArtistGroup {
                name: name.clone(),
                ids: Vec::new(),
                album_count: 0,
                first: track.clone(),
            });
            entry.ids.push(track.id);
            albums
                .entry(key)
                .or_default()
                .insert(album_key(track));
        }
        for (key, group) in &mut groups {
            group.album_count = albums.get(key).map_or(0, HashSet::len);
        }
        let mut result: Vec<_> = groups.into_values().collect();
        result.sort_by_key(|group| group.name.to_lowercase());
        result
    }

    pub(super) fn poll_thumbnails(&mut self, ctx: &egui::Context) {
        self.browse.poll(ctx);
    }

    pub(super) fn browse_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if widgets::chip(ui, "歌曲", self.view == super::View::Library).clicked() {
                self.switch_view(super::View::Library);
            }
            if widgets::chip(ui, "专辑", self.view == super::View::Albums).clicked() {
                self.switch_view(super::View::Albums);
            }
            if widgets::chip(ui, "艺术家", self.view == super::View::Artists).clicked() {
                self.switch_view(super::View::Artists);
            }
        });
    }

    pub(super) fn albums_page(&mut self, ui: &mut Ui, ids: &[u64]) {
        let groups = self.album_groups(ids);
        if let Some(focus) = self.album_focus.clone() {
            if let Some(group) = groups.iter().find(|group| group.key == focus) {
                self.album_detail(ui, group);
                return;
            }
            self.album_focus = None;
        }
        if groups.is_empty() {
            self.browse_empty(ui, "还没有专辑");
            return;
        }
        let available = ui.available_width();
        let gap = 24.0;
        let columns = grid_columns(available);
        let card_width = ((available - gap * (columns.saturating_sub(1) as f32)) / columns as f32)
            .clamp(140.0, 232.0);
        egui::ScrollArea::vertical()
            .id_salt("rplayer-albums")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for row in groups.chunks(columns) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for group in row {
                            self.album_card(ui, group, card_width);
                        }
                        if row.len() < columns {
                            self.album_import_card(ui, card_width);
                        }
                    });
                    ui.add_space(24.0);
                }
            });
    }

    pub(super) fn artists_page(&mut self, ui: &mut Ui, ids: &[u64]) {
        let groups = self.artist_groups(ids);
        if let Some(focus) = self.artist_focus.clone() {
            if let Some(group) = groups.iter().find(|group| group.name == focus) {
                self.artist_detail(ui, group);
                return;
            }
            self.artist_focus = None;
        }
        if groups.is_empty() {
            self.browse_empty(ui, "还没有艺术家");
            return;
        }
        let available = ui.available_width();
        let gap = 24.0;
        let columns = grid_columns(available);
        let card_width = ((available - gap * (columns.saturating_sub(1) as f32)) / columns as f32)
            .clamp(140.0, 232.0);
        egui::ScrollArea::vertical()
            .id_salt("rplayer-artists")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for row in groups.chunks(columns) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for group in row {
                            self.artist_card(ui, group, card_width);
                        }
                    });
                    ui.add_space(24.0);
                }
            });
    }

    fn album_import_card(&mut self, ui: &mut Ui, width: f32) {
        let card_height = width + 64.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, card_height), Sense::click());
        let cover = Rect::from_min_size(rect.min, Vec2::splat(width));
        let hovered = response.hovered();
        let painter = ui.painter();
        painter.rect_filled(cover, CornerRadius::same(2), theme::c().panel);
        painter.rect_stroke(
            cover,
            CornerRadius::same(2),
            Stroke::new(1.0_f32, theme::c().border),
            egui::StrokeKind::Inside,
        );
        let icon_color = if hovered {
            theme::c().accent
        } else {
            theme::c().faint
        };
        icons::paint(
            painter,
            Rect::from_center_size(cover.center() - Vec2::new(0.0, 12.0), Vec2::splat(24.0)),
            Icon::Plus,
            icon_color,
        );
        painter.text(
            cover.center() + Vec2::new(0.0, 14.0),
            Align2::CENTER_CENTER,
            "添加音乐",
            FontId::proportional(12.5),
            theme::c().muted,
        );
        painter.text(
            cover.center() + Vec2::new(0.0, 32.0),
            Align2::CENTER_CENTER,
            "扫描文件夹",
            FontId::proportional(9.5),
            theme::c().faint,
        );
        if response.clicked() {
            self.open_folder_picker();
        }
        response.on_hover_cursor(CursorIcon::PointingHand);
    }

    fn album_card(&mut self, ui: &mut Ui, group: &AlbumGroup, width: f32) {
        let card_height = width + 64.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, card_height), Sense::click());
        let hovered = response.hovered();
        let cover = Rect::from_min_size(rect.min, Vec2::splat(width));

        self.browse.ensure(&group.key, &group.first);
        let texture = self.browse.texture(&group.key);
        let painter = ui.painter();
        painter.rect_filled(cover, CornerRadius::same(2), theme::c().card);
        if let Some(texture) = texture {
            painter.image(
                texture.id(),
                cover,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            icons::paint(
                painter,
                Rect::from_center_size(cover.center(), Vec2::splat(44.0)),
                Icon::Music,
                theme::c().faint,
            );
        }
        if hovered {
            painter.rect_filled(cover, CornerRadius::same(2), theme::accent_alpha(0.10));
            let play = Rect::from_center_size(cover.center(), Vec2::splat(40.0));
            painter.circle_filled(play.center(), 20.0, theme::c().accent);
            icons::paint(
                painter,
                play.shrink(11.0),
                Icon::Play,
                theme::c().on_accent,
            );
        }

        painter.text(
            Pos2::new(rect.left(), cover.bottom() + 12.0),
            Align2::LEFT_TOP,
            &truncate(&group.title, 18),
            crate::ui::tokens::serif(14.0),
            theme::c().text,
        );
        painter.text(
            Pos2::new(rect.left(), cover.bottom() + 34.0),
            Align2::LEFT_TOP,
            &truncate(&group.artist, 18),
            FontId::proportional(10.5),
            theme::c().muted,
        );
        let meta = match group.year {
            Some(year) => format!("{year} · {} 首", group.ids.len()),
            None => format!("{} 首", group.ids.len()),
        };
        painter.text(
            Pos2::new(rect.left(), cover.bottom() + 50.0),
            Align2::LEFT_TOP,
            meta,
            crate::ui::tokens::mono(8.5),
            theme::c().faint,
        );

        if response.double_clicked() {
            let ids = group.ids.clone();
            self.play_queue(ui.ctx(), ids, 0);
        } else if response.clicked() {
            self.album_focus = Some(group.key.clone());
        }
        response.context_menu(|ui| {
            if widgets::text_button(ui, "查看专辑").clicked() {
                self.album_focus = Some(group.key.clone());
                ui.close_menu();
            }
            if widgets::text_button(ui, "播放专辑").clicked() {
                let ids = group.ids.clone();
                self.play_queue(ui.ctx(), ids, 0);
                ui.close_menu();
            }
            if widgets::text_button(ui, "加入播放队列").clicked() {
                self.enqueue_ids(&group.ids);
                ui.close_menu();
            }
        });
        response.on_hover_cursor(CursorIcon::PointingHand);
    }

    fn artist_card(&mut self, ui: &mut Ui, group: &ArtistGroup, width: f32) {
        let card_height = width + 64.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, card_height), Sense::click());
        let hovered = response.hovered();
        let cover = Rect::from_min_size(rect.min, Vec2::splat(width));

        let key = album_key(&group.first);
        self.browse.ensure(&key, &group.first);
        let texture = self.browse.texture(&key);
        let painter = ui.painter();
        painter.rect_filled(cover, CornerRadius::same(2), theme::c().card);
        if let Some(texture) = texture {
            painter.image(
                texture.id(),
                cover,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            let initial = group.name.chars().next().unwrap_or('?').to_string();
            painter.text(
                cover.center(),
                Align2::CENTER_CENTER,
                initial,
                crate::ui::tokens::serif(52.0),
                theme::c().accent,
            );
        }
        if hovered {
            painter.rect_filled(cover, CornerRadius::same(2), theme::accent_alpha(0.10));
            let play = Rect::from_center_size(cover.center(), Vec2::splat(40.0));
            painter.circle_filled(play.center(), 20.0, theme::c().accent);
            icons::paint(
                painter,
                play.shrink(11.0),
                Icon::Play,
                theme::c().on_accent,
            );
        }

        painter.text(
            Pos2::new(rect.left(), cover.bottom() + 12.0),
            Align2::LEFT_TOP,
            &truncate(&group.name, 16),
            crate::ui::tokens::serif(14.0),
            theme::c().text,
        );
        painter.text(
            Pos2::new(rect.left(), cover.bottom() + 36.0),
            Align2::LEFT_TOP,
            format!("{} 张专辑 · {} 首", group.album_count, group.ids.len()),
            crate::ui::tokens::mono(8.5),
            theme::c().faint,
        );

        if response.double_clicked() {
            let ids = group.ids.clone();
            self.play_queue(ui.ctx(), ids, 0);
        } else if response.clicked() {
            self.artist_focus = Some(group.name.clone());
        }
        response.context_menu(|ui| {
            if widgets::text_button(ui, "查看艺术家").clicked() {
                self.artist_focus = Some(group.name.clone());
                ui.close_menu();
            }
            if widgets::text_button(ui, "播放全部").clicked() {
                let ids = group.ids.clone();
                self.play_queue(ui.ctx(), ids, 0);
                ui.close_menu();
            }
            if widgets::text_button(ui, "加入播放队列").clicked() {
                self.enqueue_ids(&group.ids);
                ui.close_menu();
            }
        });
        response.on_hover_cursor(CursorIcon::PointingHand);
    }

    fn album_detail(&mut self, ui: &mut Ui, group: &AlbumGroup) {
        ui.horizontal(|ui| {
            if widgets::secondary_button(ui, "← 返回专辑").clicked() {
                self.album_focus = None;
            }
        });
        ui.add_space(14.0);

        self.browse.ensure(&group.key, &group.first);
        let texture = self.browse.texture(&group.key);
        ui.horizontal(|ui| {
            let cover_size = 156.0;
            let (cover, _) =
                ui.allocate_exact_size(Vec2::splat(cover_size), Sense::hover());
            ui.painter()
                .rect_filled(cover, CornerRadius::same(2), theme::c().card);
            if let Some(texture) = texture {
                ui.painter().image(
                    texture.id(),
                    cover,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                icons::paint(
                    ui.painter(),
                    Rect::from_center_size(cover.center(), Vec2::splat(42.0)),
                    Icon::Music,
                    theme::c().faint,
                );
            }

            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("ALBUM")
                        .font(crate::ui::tokens::mono(9.0))
                        .color(theme::c().faint),
                );
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(&group.title)
                        .font(crate::ui::tokens::serif(42.0))
                        .color(theme::c().text),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&group.artist)
                        .size(12.0)
                        .color(theme::c().muted),
                );
                let year = group
                    .year
                    .map(|year| format!(" · {year}"))
                    .unwrap_or_default();
                ui.label(
                    egui::RichText::new(format!(
                        "{} 首 · {}{year}",
                        group.ids.len(),
                        crate::model::format_duration(group.duration)
                    ))
                    .font(crate::ui::tokens::mono(9.0))
                    .color(theme::c().faint),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if widgets::primary_button_icon(ui, crate::ui::icons::Icon::Play, "播放全部").clicked() {
                        self.play_queue(ui.ctx(), group.ids.clone(), 0);
                    }
                    if widgets::secondary_button(ui, "加入队列").clicked() {
                        self.enqueue_ids(&group.ids);
                    }
                });
            });
        });
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(8.0);

        let queue = group.ids.clone();
        egui::ScrollArea::vertical()
            .id_salt("rplayer-album-tracks")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, id) in queue.iter().enumerate() {
                    self.browse_track_row(ui, *id, index, &queue);
                }
            });
    }

    fn artist_detail(&mut self, ui: &mut Ui, group: &ArtistGroup) {
        ui.horizontal(|ui| {
            if widgets::secondary_button(ui, "← 返回艺术家").clicked() {
                self.artist_focus = None;
            }
        });
        ui.add_space(14.0);

        let key = album_key(&group.first);
        self.browse.ensure(&key, &group.first);
        let texture = self.browse.texture(&key);
        ui.horizontal(|ui| {
            let cover_size = 118.0;
            let (cover, _) = ui.allocate_exact_size(Vec2::splat(cover_size), Sense::hover());
            ui.painter()
                .rect_filled(cover, CornerRadius::same(2), theme::c().card);
            if let Some(texture) = texture {
                ui.painter().image(
                    texture.id(),
                    cover,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                ui.painter().text(
                    cover.center(),
                    Align2::CENTER_CENTER,
                    group.name.chars().next().unwrap_or('?').to_string(),
                    crate::ui::tokens::serif(40.0),
                    theme::c().accent,
                );
            }
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("ARTIST")
                        .font(crate::ui::tokens::mono(9.0))
                        .color(theme::c().faint),
                );
                ui.label(
                    egui::RichText::new(&group.name)
                        .font(crate::ui::tokens::serif(38.0))
                        .color(theme::c().text),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} 张专辑 · {} 首歌曲",
                        group.album_count,
                        group.ids.len()
                    ))
                    .size(11.0)
                    .color(theme::c().muted),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if widgets::primary_button_icon(ui, crate::ui::icons::Icon::Play, "播放全部").clicked() {
                        self.play_queue(ui.ctx(), group.ids.clone(), 0);
                    }
                    if widgets::secondary_button(ui, "加入队列").clicked() {
                        self.enqueue_ids(&group.ids);
                    }
                });
            });
        });
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(8.0);

        let queue = group.ids.clone();
        egui::ScrollArea::vertical()
            .id_salt("rplayer-artist-tracks")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, id) in queue.iter().enumerate() {
                    self.browse_track_row(ui, *id, index, &queue);
                }
            });
    }

    fn browse_track_row(&mut self, ui: &mut Ui, id: u64, index: usize, queue: &[u64]) {
        let Some(track) = self.library.track(id).cloned() else {
            return;
        };
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(8), theme::c().card);
        }
        ui.painter().text(
            Pos2::new(rect.left() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            format!("{:02}", index + 1),
            FontId::monospace(10.0),
            theme::c().faint,
        );
        let title_clip = Rect::from_min_max(
            Pos2::new(rect.left() + 38.0, rect.top()),
            Pos2::new(rect.right() - 62.0, rect.bottom()),
        );
        ui.painter().with_clip_rect(title_clip).text(
            Pos2::new(rect.left() + 38.0, rect.center().y),
            Align2::LEFT_CENTER,
            &track.title,
            FontId::proportional(13.0),
            theme::c().text,
        );
        ui.painter().text(
            Pos2::new(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            crate::model::format_duration(track.duration),
            FontId::proportional(11.0),
            theme::c().faint,
        );
        if response.clicked() {
            self.play_queue(ui.ctx(), queue.to_vec(), index);
        }
        response.on_hover_cursor(CursorIcon::PointingHand);
    }

    fn enqueue_ids(&mut self, ids: &[u64]) {
        let added = self.append_to_queue(ids);
        if added > 0 {
            self.toast(
                format!("已加入播放队列：{added} 首"),
                super::ToastKind::Info,
            );
        }
    }

    fn browse_empty(&self, ui: &mut Ui, text: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            widgets::empty_illustration(ui, 96.0);
            ui.add_space(14.0);
            ui.label(
                egui::RichText::new(text)
                    .size(17.0)
                    .strong()
                    .color(theme::c().text),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("添加音乐文件夹或换个搜索词")
                    .size(12.0)
                    .color(theme::c().muted),
            );
        });
    }
}

fn album_key(track: &Track) -> String {
    format!(
        "{}|{}",
        track.artist.trim().to_lowercase(),
        track.album.trim().to_lowercase()
    )
}

fn display_album(track: &Track) -> String {
    if track.album.trim().is_empty() || track.album == "未知专辑" {
        "未知专辑".to_string()
    } else {
        track.album.clone()
    }
}

fn display_artist(track: &Track) -> String {
    if track.artist.trim().is_empty() || track.artist == "未知艺术家" {
        "未知艺术家".to_string()
    } else {
        track.artist.clone()
    }
}

fn truncate(text: &str, max: usize) -> String {
    let mut chars = text.chars();
    let value: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{value}…")
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::grid_columns;

    #[test]
    fn album_grid_uses_five_columns_at_desktop_widths() {
        assert_eq!(grid_columns(1200.0), 5);
        assert_eq!(grid_columns(860.0), 4);
        assert_eq!(grid_columns(520.0), 3);
    }
}
