use super::*;

impl RPlayerApp {
    pub(crate) fn error_card(&mut self, ui: &mut egui::Ui) {
        if self.error_card_dismissed || self.scan_errors.is_empty() {
            return;
        }
        let raw = self.scan_errors[0].clone();
        let (path, message) = match raw.split_once(": ") {
            Some((path, message)) => (path.to_string(), message.to_string()),
            None => (String::new(), raw.clone()),
        };
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        let elapsed = self
            .scan_finished_at
            .map(|at| at.elapsed().as_secs())
            .unwrap_or(0);
        let when = if elapsed < 60 {
            "刚刚".to_string()
        } else if elapsed < 3600 {
            format!("{} 分钟前", elapsed / 60)
        } else {
            format!("{} 小时前", elapsed / 3600)
        };

        let open = self.error_card_open;
        let t = ui.ctx().animate_bool_with_time(
            egui::Id::new("rplayer-error-card"),
            open,
            0.28,
        );
        if open && t < 1.0 {
            ui.ctx().request_repaint();
        }

        let width = ui.available_width();
        let collapsed = 52.0;
        let height = collapsed + 84.0 * t;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(8), theme::c().card);
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height())),
            0.0,
            theme::c().danger,
        );
        painter.rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.0_f32, theme::c().border_soft),
            egui::StrokeKind::Inside,
        );

        let icon_rect = Rect::from_min_size(rect.left_top() + Vec2::new(14.0, 16.0), Vec2::splat(14.0));
        icons::paint(painter, icon_rect, Icon::Warning, theme::c().danger);
        painter.text(
            rect.left_top() + Vec2::new(36.0, 12.0),
            Align2::LEFT_TOP,
            format!("无法读取「{name}」"),
            FontId::proportional(12.5),
            theme::c().text,
        );
        let detail = if self.scan_errors.len() > 1 {
            format!("{message} · 另有 {} 个文件", self.scan_errors.len() - 1)
        } else {
            message.clone()
        };
        painter.text(
            rect.left_top() + Vec2::new(36.0, 31.0),
            Align2::LEFT_TOP,
            &detail,
            FontId::proportional(9.5),
            theme::c().muted,
        );

        if open {
            let alpha = t.clamp(0.0, 1.0);
            let dim = |color: egui::Color32| {
                egui::Color32::from_rgba_unmultiplied(
                    color.r(),
                    color.g(),
                    color.b(),
                    (f32::from(color.a()) * alpha) as u8,
                )
            };
            painter.text(
                rect.left_top() + Vec2::new(36.0, 56.0),
                Align2::LEFT_TOP,
                "路径",
                FontId::monospace(8.5),
                dim(theme::c().faint),
            );
            painter.text(
                rect.left_top() + Vec2::new(66.0, 56.0),
                Align2::LEFT_TOP,
                &path,
                FontId::monospace(9.0),
                dim(theme::c().muted),
            );
            painter.text(
                rect.left_top() + Vec2::new(36.0, 74.0),
                Align2::LEFT_TOP,
                format!("DIAGNOSTIC   {message} · 已重试 0 次 · {when}"),
                FontId::monospace(8.5),
                dim(theme::c().faint),
            );
        }

        // header click toggles the card
        let header_rect = Rect::from_min_size(rect.left_top(), Vec2::new(width - 120.0, collapsed));
        let header = ui.interact(header_rect, egui::Id::new("rplayer-error-head"), Sense::click());
        if header.clicked() {
            self.error_card_open = !self.error_card_open;
        }
        header.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "错误详情"));

        let more = if open { "收起" } else { "详情" };
        let more_rect = Rect::from_min_size(
            rect.right_top() + Vec2::new(-108.0, 14.0),
            Vec2::new(48.0, 26.0),
        );
        let more_response = ui.put(more_rect, egui::Button::new(RichText::new(more).size(11.0)).frame(false));
        if more_response.clicked() {
            self.error_card_open = !self.error_card_open;
        }

        let close_rect = Rect::from_min_size(
            rect.right_top() + Vec2::new(-34.0, 14.0),
            Vec2::splat(26.0),
        );
        if ui
            .put(close_rect, egui::Button::new(RichText::new("×").size(16.0)).frame(false))
            .on_hover_text("关闭错误提示")
            .clicked()
        {
            self.error_card_dismissed = true;
        }

        if open && t > 0.4 {
            let y = rect.bottom() - 32.0;
            let mut x = rect.left() + 36.0;
            let reveal = Rect::from_min_size(Pos2::new(x, y), Vec2::new(112.0, 26.0));
            if ui.put(reveal, egui::Button::new(RichText::new("在文件夹中显示").size(11.0)).frame(false)).clicked() {
                if !path.is_empty() {
                    self.reveal_in_file_manager(std::path::Path::new(&path));
                }
            }
            x += 120.0;
            let copy = Rect::from_min_size(Pos2::new(x, y), Vec2::new(108.0, 26.0));
            if ui.put(copy, egui::Button::new(RichText::new("复制诊断信息").size(11.0)).frame(false)).clicked() {
                ui.ctx().copy_text(format!("{raw}\n诊断：{message}\n时间：{when}"));
            }
            x += 116.0;
            let retry = Rect::from_min_size(Pos2::new(x, y), Vec2::new(56.0, 26.0));
            if ui.put(retry, egui::Button::new(RichText::new("重试").size(11.0)).frame(false)).clicked() {
                self.error_card_dismissed = true;
                self.start_scan();
            }
        }

        ui.add_space(14.0);
    }

    pub(crate) fn central_panel(&mut self, ctx: &egui::Context, ids: &[u64], total_seconds: f64) {
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::c().bg).inner_margin(Margin {
                left: 32,
                right: 28,
                top: 22,
                bottom: 12,
            }))
            .show(ctx, |ui| {
                let dt = ctx.input(|input| input.stable_dt).min(0.1);
                if crate::ui::motion::reduced() {
                    self.view_fade = 1.0;
                } else {
                    self.view_fade = (self.view_fade + dt * 4.0).min(1.0);
                }
                if self.view_fade < 1.0 {
                    ctx.request_repaint();
                }
                let fade = 1.0 - (1.0 - self.view_fade).powi(3);
                ui.add_space((1.0 - fade) * 10.0);
                ui.multiply_opacity(fade);
                self.error_card(ui);

                if self.view == View::Albums && self.album_focus.is_some() {
                    self.albums_page(ui, ids);
                    return;
                }
                if self.view == View::Artists && self.artist_focus.is_some() {
                    self.artists_page(ui, ids);
                    return;
                }

                self.central_header(ui, ids, total_seconds);
                ui.add_space(10.0);
                if matches!(self.view, View::Library | View::Albums | View::Artists) {
                    self.browse_tabs(ui);
                    ui.add_space(12.0);
                } else {
                    ui.add_space(4.0);
                }

                if self.view == View::Folders {
                    self.folders_page(ui);
                    return;
                }
                if self.view == View::Albums {
                    self.albums_page(ui, ids);
                    return;
                }
                if self.view == View::Artists {
                    self.artists_page(ui, ids);
                    return;
                }

                if self.library.tracks.is_empty() {
                    self.empty_library_state(ui);
                    return;
                }

                if self.batch_mode {
                    self.batch_bar(ui, ids);
                    ui.add_space(8.0);
                }

                Frame::NONE
                    .fill(theme::c().bg)
                    .inner_margin(Margin::symmetric(0, 0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let show_album =
                            crate::app::layout::Tier::of_ctx(ui.ctx()).shows_album_column();
                        let columns =
                            Columns::for_width_opt(ui.available_width(), show_album);
                        self.table_header(ui, columns);

                        if ids.is_empty() {
                            self.no_results_state(ui);
                        } else {
                            self.track_table(ui, ids, columns);
                        }
                    });
            });
    }

}
