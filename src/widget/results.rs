use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Margin, Rect},
    style::{Style, Stylize as _},
    symbols,
    text::Line,
    widgets::{Clear, Paragraph, Row, ScrollbarOrientation, StatefulWidget, Table, Widget},
    Frame,
};

use crate::{
    app::{Context, LoadType, Mode},
    title,
    widget::sort::SortDir,
};

use super::{border_block, centered_rect, Corner, VirtualStatefulTable};

pub struct ResultsWidget {
    pub table: VirtualStatefulTable,
    control_space: bool,
    visual_anchor: usize,
    // draw_count: u64,
}

impl ResultsWidget {
    pub fn reset(&mut self) {
        self.table.select(0);
        *self.table.state.offset_mut() = 0;
    }

    fn try_select_toggle(&self, ctx: &mut Context, sel: usize) {
        if let Some(item) = ctx.results.response.items.get(sel) {
            if let Some(p) = ctx.batch.iter().position(|s| s.id == item.id) {
                ctx.batch.remove(p);
            } else {
                ctx.batch.push(item.to_owned());
            }
        }
    }
}

impl Default for ResultsWidget {
    fn default() -> Self {
        ResultsWidget {
            table: VirtualStatefulTable::new(),
            control_space: false,
            visual_anchor: 0,
            // draw_count: 0,
        }
    }
}

impl super::Widget for ResultsWidget {
    fn draw(&mut self, f: &mut Frame, ctx: &Context, area: Rect) {
        let buf = f.buffer_mut();
        let focus_color = match ctx.mode {
            Mode::Normal | Mode::KeyCombo(_) => ctx.theme.border_focused_color,
            _ => ctx.theme.border_color,
        };
        let header: Row = ctx.results.table.headers.clone().into();
        let header = header.fg(focus_color).underlined();

        Clear.render(area, buf);
        let items: Vec<Row> = match &ctx.load_type {
            Some(loadtype) => {
                let message = format!("{}…", loadtype);
                let load_area = centered_rect(message.len() as u16, 1, area);
                Paragraph::new(message).render(load_area, buf);
                vec![]
            }
            _ => ctx
                .results
                .table
                .rows
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
        };

        let sb = super::scrollbar(ctx, ScrollbarOrientation::VerticalRight).begin_symbol(Some(""));
        let sb_area = area.inner(&Margin {
            vertical: 1,
            horizontal: 0,
        });

        let num_items = items.len();
        let first_item = (ctx.page - 1) * 75;
        let focused = matches!(ctx.mode, Mode::Normal | Mode::KeyCombo(_));
        let table = Table::new(items, ctx.results.table.binding.to_owned())
            .header(header)
            .block(border_block(&ctx.theme, focused).title(title!(
                "Results {}-{} ({} total): Page {}/{}",
                first_item + 1,
                num_items + first_item,
                ctx.results.response.total_results,
                ctx.page,
                ctx.results.response.last_page,
            )))
            .highlight_style(Style::default().bg(ctx.theme.hl_bg));

        super::scroll_padding(
            self.table.selected().unwrap_or(0),
            area.height as usize,
            3,
            num_items,
            ctx.config.scroll_padding,
            self.table.state.offset_mut(),
        );

        StatefulWidget::render(table, area, buf, &mut self.table.state);
        StatefulWidget::render(
            sb,
            sb_area,
            buf,
            &mut self.table.scrollbar_state.content_length(num_items),
        );

        if ctx.load_type.is_none() && num_items == 0 {
            let center = centered_rect(10, 1, area);
            Paragraph::new("No results").render(center, buf);
        }

        if area.height >= 3 {
            if let Some(visible_items) = ctx.results.response.items.get(self.table.state.offset()..)
            {
                let selected_ids: Vec<String> =
                    ctx.batch.clone().into_iter().map(|i| i.id).collect();
                let vert_left = ctx.theme.border.to_border_set().vertical_left;
                let lines = visible_items
                    .iter()
                    .map(|i| {
                        Line::from(
                            match selected_ids.contains(&i.id) {
                                true => symbols::border::QUADRANT_BLOCK,
                                false => vert_left,
                            }
                            .to_owned(),
                        )
                    })
                    .collect::<Vec<Line>>();
                let para = Paragraph::new(lines);
                let para_area = Rect::new(area.x, area.y + 2, 1, area.height - 3);
                para.render(para_area, buf);
            }
        }

        let dl_src = title!(
            "dl: {}, src: {}",
            ctx.client.to_string(),
            ctx.src.to_string()
        );
        if let Some((tr, area)) = Corner::TopRight.try_title(dl_src, area, true) {
            f.render_widget(tr, area);
        }

        if !ctx.last_key.is_empty() {
            let key_str = title!(ctx.last_key);
            if let Some((br, area)) = Corner::BottomRight.try_title(key_str, area, true) {
                f.render_widget(br, area);
            }
        }

        // if let Some((bl, area)) =
        //     Corner::BottomLeft.try_title(format!("{} draws", self.draw_count), area, false)
        // {
        //     f.render_widget(bl, area);
        //     self.draw_count += 1;
        // }
    }

    fn handle_event(&mut self, ctx: &mut Context, e: &Event) {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = e
        {
            use KeyCode::*;
            match (code, modifiers) {
                (Char('c'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Category;
                }
                (Char('s'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Sort(SortDir::Desc);
                }
                (Char('S'), &KeyModifiers::SHIFT) => {
                    ctx.mode = Mode::Sort(SortDir::Asc);
                }
                (Char('f'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Filter;
                }
                (Char('t'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Theme;
                }
                (Char('/') | Char('i'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Search;
                }
                (Char('p'), &KeyModifiers::CONTROL) => {
                    ctx.mode = Mode::Page;
                }
                (Char('p') | Char('h') | Left, &KeyModifiers::NONE) => {
                    if ctx.page > 1 {
                        ctx.page -= 1;
                        ctx.mode = Mode::Loading(LoadType::Searching);
                    }
                }
                (Char('n') | Char('l') | Right, &KeyModifiers::NONE) => {
                    if ctx.page < ctx.results.response.last_page {
                        ctx.page += 1;
                        ctx.mode = Mode::Loading(LoadType::Searching);
                    }
                }
                (Char('r'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Loading(LoadType::Searching);
                }
                (Char('q'), &KeyModifiers::NONE) => {
                    ctx.quit();
                }
                (Char('j') | KeyCode::Down, &KeyModifiers::NONE) => {
                    let prev = self.table.selected().unwrap_or(0);
                    let selected = self.table.next(ctx.results.response.items.len(), 1);
                    if self.control_space && prev != selected {
                        self.try_select_toggle(
                            ctx,
                            match selected <= self.visual_anchor {
                                true => prev,
                                false => selected,
                            },
                        );
                    }
                }
                (Char('k') | KeyCode::Up, &KeyModifiers::NONE) => {
                    let prev = self.table.selected().unwrap_or(0);
                    let selected = self.table.next(ctx.results.response.items.len(), -1);
                    if self.control_space && prev != selected {
                        self.try_select_toggle(
                            ctx,
                            match selected >= self.visual_anchor {
                                true => prev,
                                false => selected,
                            },
                        );
                    }
                }
                (Char('J'), &KeyModifiers::SHIFT) => {
                    self.table.next(ctx.results.response.items.len(), 4);
                }
                (Char('K'), &KeyModifiers::SHIFT) => {
                    self.table.next(ctx.results.response.items.len(), -4);
                }
                (Char('G'), &KeyModifiers::SHIFT) => {
                    self.table
                        .select(ctx.results.response.items.len().saturating_sub(1));
                }
                (Char('g'), &KeyModifiers::NONE) => {
                    self.table.select(0);
                }
                (Char('H') | Char('P'), &KeyModifiers::SHIFT) => {
                    if ctx.page != 1 {
                        ctx.page = 1;
                        ctx.mode = Mode::Loading(LoadType::Searching);
                    }
                }
                (Char('L') | Char('N'), &KeyModifiers::SHIFT) => {
                    if ctx.page != ctx.results.response.last_page
                        && ctx.results.response.last_page > 0
                    {
                        ctx.page = ctx.results.response.last_page;
                        ctx.mode = Mode::Loading(LoadType::Searching);
                    }
                }
                (Enter, &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Loading(LoadType::Downloading);
                }
                (Char('s'), &KeyModifiers::CONTROL) => {
                    ctx.mode = Mode::Sources;
                }
                (Char('d'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::Clients;
                }
                (Char('u'), &KeyModifiers::NONE) => {
                    ctx.mode = Mode::User;
                }
                (Char('o'), &KeyModifiers::NONE) => {
                    let link = ctx
                        .results
                        .response
                        .items
                        .get(self.table.state.selected().unwrap_or(0))
                        .map(|item| item.post_link.clone())
                        .unwrap_or("https://nyaa.si".to_owned());
                    let res = open::that_detached(link.clone());
                    if let Err(e) = res {
                        ctx.show_error(format!("Failed to open {}:\n{}", link, e));
                    } else {
                        ctx.notify(format!("Opened {}", link));
                    }
                }
                (Char('y'), &KeyModifiers::NONE) => ctx.mode = Mode::KeyCombo("y".to_string()),
                (Char(' '), &KeyModifiers::CONTROL) => {
                    self.control_space = !self.control_space;
                    if self.control_space {
                        ctx.notify("Entered VISUAL mode");
                        self.visual_anchor = self.table.selected().unwrap_or(0);
                        self.try_select_toggle(ctx, self.visual_anchor);
                    } else {
                        ctx.notify("Exited VISUAL mode");
                        self.visual_anchor = 0;
                    }
                }
                (Char(' '), &KeyModifiers::NONE) => {
                    if let Some(sel) = self.table.state.selected() {
                        if let Some(item) = &mut ctx.results.response.items.get_mut(sel) {
                            if let Some(p) = ctx.batch.iter().position(|s| s.id == item.id) {
                                ctx.batch.remove(p);
                            } else {
                                ctx.batch.push(item.to_owned());
                            }
                        }
                    }
                }
                (Tab | BackTab, _) => {
                    ctx.mode = Mode::Batch;
                }
                (Esc, &KeyModifiers::NONE) => {
                    if self.control_space {
                        ctx.notify("Exited VISUAL mode");
                        self.visual_anchor = 0;
                        self.control_space = false;
                    } else {
                        ctx.dismiss_notifications();
                    }
                }
                _ => {}
            }
        }
    }

    fn get_help() -> Option<Vec<(&'static str, &'static str)>> {
        Some(vec![
            ("Enter", "Confirm"),
            ("Esc", "Dismiss notification"),
            ("q", "Exit App"),
            ("g/G", "Goto Top/Bottom"),
            ("k, ↑", "Up"),
            ("j, ↓", "Down"),
            ("K, J", "Up/Down 4 items"),
            ("n, l, →", "Next Page"),
            ("p, h, ←", "Prev Page"),
            ("N, L", "Last Page"),
            ("P, H", "First Page"),
            ("r", "Reload"),
            ("o", "Open in browser"),
            ("yt, ym, yp, yi", "Copy torrent/magnet/post/imdb id"),
            ("Space", "Toggle item for batch download"),
            ("Ctrl-Space", "Multi-line select torrents"),
            ("Tab/Shift-Tab", "Switch to Batches"),
            ("/, i", "Search"),
            ("c", "Categories"),
            ("f", "Filters"),
            ("s", "Sort"),
            ("S", "Sort reversed"),
            ("t", "Themes"),
            ("u", "Filter by User"),
            ("d", "Select download client"),
            ("Ctrl-p", "Goto page"),
            ("Ctrl-s", "Select source"),
        ])
    }
}
