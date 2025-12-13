//! Sidebar component for Safe context (chain, address, version, info)

use eframe::egui;
use crate::hasher::SafeInfo;
use crate::state::{SafeContext, SidebarState, SAFE_VERSIONS};
use crate::ui;

/// Sidebar action returned after rendering
pub enum SidebarAction {
    None,
    FetchDetails,
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                
                // Header with collapse button
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("Safe Details").size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("◀").on_hover_text("Collapse sidebar").clicked() {
                            sidebar.collapsed = true;
                        }
                    });
                });
                ui.separator();
                ui.add_space(5.0);
                
                // Chain selection
                ui.horizontal(|ui| {
                    ui.label("Chain:");
                    egui::ComboBox::from_id_salt("sidebar_chain")
                        .selected_text(&safe_ctx.chain_name)
                        .width(150.0)
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
                ui.add_space(8.0);
                
                // Safe Address with recent suggestions
                ui.label("Safe Address:");
                let addr_response = ui.add(
                    egui::TextEdit::singleline(&mut safe_ctx.safe_address)
                        .hint_text("0x...")
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
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
                
                // Cache on blur
                if addr_response.lost_focus() && !addr_response.ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    crate::state::save_safe_address(&safe_ctx.safe_address);
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
                                    for addr in &safe_ctx.recent_addresses.clone() {
                                        let response = ui.selectable_label(
                                            safe_ctx.safe_address.to_lowercase() == addr.to_lowercase(),
                                            egui::RichText::new(addr).monospace().size(11.0)
                                        );
                                        if response.clicked() {
                                            safe_ctx.safe_address = addr.clone();
                                            crate::state::save_safe_address(addr);
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
                
                ui.add_space(8.0);
                
                // Version display
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    
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
                ui.add_space(10.0);
                
                // Fetch Details button
                let is_valid_address = safe_ctx.safe_address.starts_with("0x") 
                    && safe_ctx.safe_address.len() == 42;
                
                ui.horizontal(|ui| {
                    if ui.add_enabled(
                        is_valid_address && !safe_info_loading,
                        egui::Button::new("⟳ Fetch Details")
                    ).on_hover_text("Fetch Safe info (threshold, owners, modules, nonce)")
                     .clicked() 
                    {
                        crate::state::save_safe_address(&safe_ctx.safe_address);
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
                                ui.label("└");
                                ui::address_link(ui, &safe_ctx.chain_name, &addr);
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
                                    ui.label("└");
                                    ui::address_link(ui, &safe_ctx.chain_name, &addr);
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
            .exact_width(30.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                if ui.button("▶").on_hover_text("Expand sidebar").clicked() {
                    sidebar.collapsed = false;
                }
            });
    }
    
    action
}

