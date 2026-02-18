//! Sidebar component for Safe context (chain, address, version, info)

use crate::hasher::SafeInfo;
use crate::state::{SafeContext, SidebarState, SAFE_VERSIONS};
use crate::ui;
use eframe::egui;
use safe_utils::Of;

/// Sidebar action returned after rendering
pub enum SidebarAction {
    None,
    FetchDetails,
    ClearStorage,
}

/// Render the sidebar panel
pub fn render(
    ctx: &egui::Context,
    sidebar: &mut SidebarState,
    safe_ctx: &mut SafeContext,
    safe_info: &Option<SafeInfo>,
    safe_info_loading: bool,
    chain_names: &[String],
) -> SidebarAction {
    let mut action = SidebarAction::None;

    egui::SidePanel::left("safe_context_panel")
        .resizable(true)
        .default_width(280.0)
        .min_width(60.0)
        .show_animated(ctx, !sidebar.collapsed, |ui| {
            // Footer with GitHub link and clear storage
            egui::TopBottomPanel::bottom("sidebar_footer")
                .frame(egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(8.0, 8.0))
                    .fill(ui.visuals().faint_bg_color))
                .show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {

                        // Build Info Modal
                        let modal_id = ui.make_persistent_id("build_info_modal");
                        let mut show_modal = ui.memory(|m| m.data.get_temp::<bool>(modal_id).unwrap_or(false));

                        ui.horizontal(|ui| {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("").size(20.0))
                                    .frame(false)
                            ).on_hover_text("View on GitHub").clicked() {
                                ui::open_url_new_tab("https://github.com/pripatelUK/rusty-safe");
                            }
                            if ui.link(egui::RichText::new("Build Info")).clicked() {
                                show_modal = true;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(
                                    egui::Button::new(egui::RichText::new("🗑 Delete Data").size(14.0))
                                        .frame(false)
                                ).on_hover_text("Clear cached data").clicked() {
                                    action = SidebarAction::ClearStorage;
                                }
                            });
                        });

                        // Show modal when open
                        if show_modal {
                            let screen_rect = ctx.screen_rect();
                            
                            // Dimmed background that closes modal on click
                            let bg_response = egui::Area::new(ui.make_persistent_id("build_info_bg"))
                                .order(egui::Order::Background)
                                .fixed_pos(screen_rect.min)
                                .show(ctx, |ui| {
                                    ui.allocate_response(screen_rect.size(), egui::Sense::click())
                                });
                            
                            if bg_response.inner.clicked() {
                                show_modal = false;
                            }

                            // Modal window
                            egui::Window::new("Build Info")
                                .collapsible(false)
                                .resizable(false)
                                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                                .show(ctx, |ui| {
                                    let git_hash = env!("GIT_HASH");
                                    let short_hash = if git_hash.len() > 7 { &git_hash[..7] } else { git_hash };
                                    let build_time = env!("BUILD_TIME");
                                    let short_build_time = if build_time.len() > 19 { &build_time[..19] } else { build_time };

                                    ui.label(egui::RichText::new(format!("Build Time: {}", short_build_time)));
                                    
                                    ui.horizontal(|ui| {
                                        ui.label("Commit:");
                                        if ui.link(egui::RichText::new(short_hash).monospace()).clicked() {
                                            ui::open_url_new_tab(&format!("https://github.com/pripatelUK/rusty-safe/tree/{}", git_hash));
                                        }
                                    });

                                    ui.add_space(8.0);
                                    
                                    ui.horizontal(|ui| {
                                        if ui.link("View Build Info").clicked() {
                                            ui::open_url_new_tab("/BUILD_INFO.txt");
                                        }
                                        ui.separator();
                                        if ui.link("Verify Build").clicked() {
                                            ui::open_url_new_tab("https://github.com/pripatelUK/rusty-safe/blob/main/VERIFY.md");
                                        }
                                    });
                                });
                        }

                        ui.memory_mut(|m| m.data.insert_temp(modal_id, show_modal));

                    });
                });
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(12.0);

                // Header with collapse button
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Safe Details").size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let collapse_btn = egui::Button::new(
                            egui::RichText::new("◀").size(14.0)
                        ).min_size(egui::vec2(24.0, 24.0));
                        if ui.add(collapse_btn).on_hover_text("Collapse sidebar").clicked() {
                            sidebar.collapsed = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(12.0);

                // Chain selection
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Chain:").strong());
                    egui::ComboBox::from_id_salt("sidebar_chain")
                        .selected_text(&safe_ctx.chain_name)
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for chain_name in chain_names {
                                ui.selectable_value(
                                    &mut safe_ctx.chain_name,
                                    chain_name.clone(),
                                    chain_name,
                                );
                            }
                        });
                });
                ui.add_space(12.0);
                
                // Safe Address with recent suggestions
                ui.label(egui::RichText::new("Safe Address:").strong());
                ui.add_space(4.0);
                let addr_response = ui.add(
                    egui::TextEdit::singleline(&mut safe_ctx.safe_address)
                        .hint_text("0x...")
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .margin(egui::vec2(8.0, 6.0)),
                );
                
                // Track popup visibility in memory
                let popup_id = ui.make_persistent_id("recent_addresses_popup");
                let mut show_popup = ui.memory(|m| m.data.get_temp::<bool>(popup_id).unwrap_or(false));
                
                // Show popup when input gains focus
                if addr_response.gained_focus() {
                    show_popup = true;
                }
                
                // Hide popup on Escape or when clicking outside
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    show_popup = false;
                }
                
                // Add to recent on blur (will be persisted by eframe auto-save)
                if addr_response.lost_focus() && !addr_response.ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    crate::state::add_recent_address(&mut safe_ctx.recent_addresses, &safe_ctx.safe_address);
                }
                
                // Show recent addresses popup
                if show_popup && !safe_ctx.recent_addresses.is_empty() {
                    let below_rect = egui::Rect::from_min_size(
                        addr_response.rect.left_bottom(),
                        egui::vec2(addr_response.rect.width(), 0.0),
                    );
                    
                    let area_response = egui::Area::new(popup_id)
                        .order(egui::Order::Foreground)
                        .fixed_pos(below_rect.left_top())
                        .show(ui.ctx(), |ui| {
                            egui::Frame::popup(ui.style())
                                .show(ui, |ui| {
                                    ui.set_min_width(below_rect.width());
                                    let chain_id = alloy::primitives::ChainId::of(&safe_ctx.chain_name).map(u64::from).unwrap_or(1);
                                    for addr in &safe_ctx.recent_addresses.clone() {
                                        let name = safe_ctx.address_book.get_name(addr, chain_id);
                                        let label_text = if let Some(n) = name {
                                            format!("{} ({})", addr, n)
                                        } else {
                                            addr.clone()
                                        };

                                        let response = ui.selectable_label(
                                            safe_ctx.safe_address.to_lowercase() == addr.to_lowercase(),
                                            egui::RichText::new(label_text).monospace().size(11.0)
                                        );
                                        if response.clicked() {
                                            safe_ctx.safe_address = addr.clone();
                                            show_popup = false;
                                        }
                                    }
                                });
                        });
                    
                    // Close popup if clicked outside
                    if ui.input(|i| i.pointer.any_click()) 
                        && !area_response.response.rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default()))
                        && !addr_response.rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default()))
                    {
                        show_popup = false;
                    }
                }
                
                // Store popup state
                ui.memory_mut(|m| m.data.insert_temp(popup_id, show_popup));
                
                ui.add_space(12.0);

                // Version display
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Version:").strong());
                    
                    // If we have safe_info with a valid version, show as read-only
                    let version_from_api = safe_info.as_ref()
                        .filter(|info| SAFE_VERSIONS.contains(&info.version.as_str()));
                    
                    if version_from_api.is_some() {
                        ui.add_enabled(false, 
                            egui::Button::new(&safe_ctx.safe_version)
                                .min_size(egui::vec2(80.0, 0.0))
                        );
                        ui.label(egui::RichText::new("(API)").weak().small());
                    } else {
                        egui::ComboBox::from_id_salt("sidebar_version")
                            .selected_text(&safe_ctx.safe_version)
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for version in SAFE_VERSIONS {
                                    ui.selectable_value(
                                        &mut safe_ctx.safe_version,
                                        version.to_string(),
                                        *version,
                                    );
                                }
                            });
                    }
                });
                ui.add_space(16.0);

                // Fetch Details button - more prominent
                let is_valid_address = safe_ctx.safe_address.starts_with("0x")
                    && safe_ctx.safe_address.len() == 42;

                ui.horizontal(|ui| {
                    let button = egui::Button::new(
                        egui::RichText::new("⟳ Fetch Details").size(14.0)
                    ).min_size(egui::vec2(120.0, 28.0));

                    if ui.add_enabled(is_valid_address && !safe_info_loading, button)
                        .on_hover_text("Fetch Safe info (threshold, owners, modules, nonce)")
                        .clicked()
                    {
                        crate::state::add_recent_address(&mut safe_ctx.recent_addresses, &safe_ctx.safe_address);
                        action = SidebarAction::FetchDetails;
                    }

                    if safe_info_loading {
                        ui.spinner();
                    }
                });
                
                // Safe info display
                if let Some(info) = safe_info {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);
                    
                    // Threshold with signers
                    egui::CollapsingHeader::new(
                        egui::RichText::new(format!("Threshold ({}/{})", info.threshold, info.owners.len())).strong()
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        for owner in &info.owners {
                            let addr = format!("{:?}", owner);
                            ui.horizontal(|ui| {
                                // ui.label("└");
                                let chain_id = alloy::primitives::ChainId::of(&safe_ctx.chain_name).unwrap_or(1);
                                let name = safe_ctx.address_book.get_name(&addr, chain_id);
                                ui::address_link(ui, &safe_ctx.chain_name, &addr, name);
                            });
                        }
                    });
                    
                    // Modules
                    if !info.modules.is_empty() {
                        egui::CollapsingHeader::new(
                            egui::RichText::new(format!("Modules ({})", info.modules.len())).strong()
                        )
                        .default_open(true)
                        .show(ui, |ui| {
                            for module in &info.modules {
                                let addr = format!("{:?}", module);
                                ui.horizontal(|ui| {
                                    // ui.label("└");
                                    let chain_id = alloy::primitives::ChainId::of(&safe_ctx.chain_name).unwrap_or(1);
                                    let name = safe_ctx.address_book.get_name(&addr, chain_id);
                                    ui::address_link(ui, &safe_ctx.chain_name, &addr, name);
                                });
                            }
                        });
                    }
                    
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Nonce:").weak());
                        ui.label(format!("{}", info.nonce));
                    });
                }
                
                ui.add_space(20.0);
            });
        });

    // Show expand button when collapsed
    if sidebar.collapsed {
        egui::SidePanel::left("collapsed_sidebar")
            .resizable(false)
            .exact_width(36.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                let expand_btn = egui::Button::new(egui::RichText::new("▶").size(14.0))
                    .min_size(egui::vec2(28.0, 28.0));
                if ui.add(expand_btn).on_hover_text("Expand sidebar").clicked() {
                    sidebar.collapsed = false;
                }
            });
    }

    action
}
