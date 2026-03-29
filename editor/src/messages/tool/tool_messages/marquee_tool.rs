use super::tool_prelude::*;
use crate::consts::{COLOR_OVERLAY_BLUE, COLOR_OVERLAY_BLUE_05};
use crate::messages::input_mapper::utility_types::input_mouse::ViewportPosition;
use crate::messages::portfolio::document::overlays::utility_types::OverlayContext;
use graphene_std::math::quad::Quad;

/// The Marquee Rectangle tool lets the user draw a rectangular selection mask by dragging on the
/// canvas.  When the drag is committed the selection is stored in the document's `selection_mask`
/// field (in document space) and a marching-ants outline is shown via the persistent
/// `SELECTION_MASK_OVERLAY_PROVIDER`.
///
/// This tool is designed to be used in Mask editing mode (see `DocumentMessage::EnterMaskMode`),
/// but it works independently as well – it simply updates the stored selection mask each time the
/// user finishes a drag, replacing any previous selection.
#[derive(Default, ExtractField)]
pub struct MarqueeTool {
	fsm_state: MarqueeToolFsmState,
	data: MarqueeToolData,
	options: MarqueeOptions,
}

/// Options exposed in the tool controls bar.
#[derive(Clone, Debug)]
pub struct MarqueeOptions {
	/// When `true` the new selection is *added* to the existing mask;
	/// when `false` (default) it *replaces* it.
	pub add_to_selection: bool,
}

impl Default for MarqueeOptions {
	fn default() -> Self {
		Self { add_to_selection: false }
	}
}

#[impl_message(Message, ToolMessage, Marquee)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MarqueeToolMessage {
	// Standard messages
	Abort,
	WorkingColorChanged,
	Overlays { context: OverlayContext },

	// Tool-specific messages
	DragStart,
	DragStop,
	PointerMove,

	/// Update a single tool option.
	UpdateOptions { options: MarqueeOptionsUpdate },
}

/// Individual option update payloads.
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MarqueeOptionsUpdate {
	AddToSelection(bool),
}

// ── ToolMetadata ──────────────────────────────────────────────────────────────

impl ToolMetadata for MarqueeTool {
	fn icon_name(&self) -> String {
		// TODO: Replace with a dedicated "RasterMarqueeRectTool" SVG icon once the branding assets include one.
		"RasterBrushTool".into()
	}

	fn tooltip_label(&self) -> String {
		"Marquee Rectangle Tool".into()
	}

	fn tool_type(&self) -> ToolType {
		ToolType::MarqueeRect
	}
}

// ── LayoutHolder (tool options bar) ───────────────────────────────────────────

impl LayoutHolder for MarqueeTool {
	fn layout(&self) -> Layout {
		let widgets = vec![
			CheckboxInput::new(self.options.add_to_selection)
				.on_update(|checkbox: &CheckboxInput| {
					MarqueeToolMessage::UpdateOptions {
						options: MarqueeOptionsUpdate::AddToSelection(checkbox.checked),
					}
					.into()
				})
				.widget_instance(),
		];
		Layout(vec![LayoutGroup::row(widgets)])
	}
}

// ── MessageHandler ────────────────────────────────────────────────────────────

#[message_handler_data]
impl<'a> MessageHandler<ToolMessage, &mut ToolActionMessageContext<'a>> for MarqueeTool {
	fn process_message(&mut self, message: ToolMessage, responses: &mut VecDeque<Message>, context: &mut ToolActionMessageContext<'a>) {
		if let ToolMessage::Marquee(MarqueeToolMessage::UpdateOptions { options }) = message {
			match options {
				MarqueeOptionsUpdate::AddToSelection(v) => self.options.add_to_selection = v,
			}
			return;
		}
		self.fsm_state.process_event(message, &mut self.data, context, &self.options, responses, true);
	}

	fn actions(&self) -> ActionList {
		match self.fsm_state {
			MarqueeToolFsmState::Ready => actions!(MarqueeToolMessageDiscriminant; DragStart, Abort),
			MarqueeToolFsmState::Drawing => actions!(MarqueeToolMessageDiscriminant; DragStop, PointerMove, Abort),
		}
	}
}

// ── ToolTransition ────────────────────────────────────────────────────────────

impl ToolTransition for MarqueeTool {
	fn event_to_message_map(&self) -> EventToMessageMap {
		EventToMessageMap {
			tool_abort: Some(MarqueeToolMessage::Abort.into()),
			overlay_provider: Some(|context| MarqueeToolMessage::Overlays { context }.into()),
			..Default::default()
		}
	}
}

// ── FSM state ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MarqueeToolFsmState {
	#[default]
	Ready,
	Drawing,
}

/// Runtime data kept by the tool across state transitions.
#[derive(Clone, Debug, Default)]
pub struct MarqueeToolData {
	/// Viewport-space position where the current drag started.
	drag_start: ViewportPosition,
	/// Viewport-space position of the most recent pointer event.
	drag_current: ViewportPosition,
}

// ── Fsm impl ──────────────────────────────────────────────────────────────────

impl Fsm for MarqueeToolFsmState {
	type ToolData = MarqueeToolData;
	type ToolOptions = MarqueeOptions;

	fn transition(
		self,
		event: ToolMessage,
		tool_data: &mut Self::ToolData,
		context: &mut ToolActionMessageContext,
		options: &Self::ToolOptions,
		responses: &mut VecDeque<Message>,
	) -> Self {
		let ToolActionMessageContext { document, input, .. } = context;

		let ToolMessage::Marquee(event) = event else { return self };

		match (self, event) {
			// ── Overlay rendering ────────────────────────────────────────────
			(_, MarqueeToolMessage::Overlays { context: mut overlay_context }) => {
				// While the user is dragging, draw an in-progress selection rectangle.
				if self == MarqueeToolFsmState::Drawing {
					let quad = Quad::from_box([tool_data.drag_start, tool_data.drag_current]);
					overlay_context.dashed_quad(quad, Some(COLOR_OVERLAY_BLUE), Some(COLOR_OVERLAY_BLUE_05), Some(4.), Some(4.), Some(0.5));
				}
				self
			}

			// ── Ready → Drawing ──────────────────────────────────────────────
			(MarqueeToolFsmState::Ready, MarqueeToolMessage::DragStart) => {
				let pos = input.mouse.position;
				tool_data.drag_start = pos;
				tool_data.drag_current = pos;
				responses.add(OverlaysMessage::Draw);
				MarqueeToolFsmState::Drawing
			}

			// ── Drawing → Drawing (pointer movement) ─────────────────────────
			(MarqueeToolFsmState::Drawing, MarqueeToolMessage::PointerMove) => {
				tool_data.drag_current = input.mouse.position;
				responses.add(OverlaysMessage::Draw);
				MarqueeToolFsmState::Drawing
			}

			// ── Drawing → Ready (commit selection) ───────────────────────────
			(MarqueeToolFsmState::Drawing, MarqueeToolMessage::DragStop) => {
				tool_data.drag_current = input.mouse.position;

				let vp_start = tool_data.drag_start;
				let vp_end = tool_data.drag_current;

				// Convert the viewport-space rectangle to document space so it stays valid
				// when the user pans or zooms.
				let doc_to_viewport = document.metadata().document_to_viewport;
				let viewport_to_doc = doc_to_viewport.inverse();
				let doc_start = viewport_to_doc.transform_point2(vp_start);
				let doc_end = viewport_to_doc.transform_point2(vp_end);
				let rect = [doc_start.min(doc_end), doc_start.max(doc_end)];

				if options.add_to_selection {
					// Expand the existing mask to encompass the new rectangle.
					if let Some(existing) = document.selection_mask {
						let new_min = existing[0].min(rect[0]);
						let new_max = existing[1].max(rect[1]);
						responses.add(DocumentMessage::SetSelectionMask { rect: [new_min, new_max] });
					} else {
						responses.add(DocumentMessage::SetSelectionMask { rect });
					}
				} else {
					responses.add(DocumentMessage::SetSelectionMask { rect });
				}

				responses.add(OverlaysMessage::Draw);
				MarqueeToolFsmState::Ready
			}

			// ── Abort ────────────────────────────────────────────────────────
			(_, MarqueeToolMessage::Abort) => {
				responses.add(OverlaysMessage::Draw);
				MarqueeToolFsmState::Ready
			}

			// ── No-op ─────────────────────────────────────────────────────────
			_ => self,
		}
	}

	fn update_hints(&self, responses: &mut VecDeque<Message>) {
		let hint_data = match self {
			MarqueeToolFsmState::Ready => HintData(vec![
				HintGroup(vec![HintInfo::mouse(MouseMotion::LmbDrag, "Draw Selection")]),
				HintGroup(vec![HintInfo::keys([Key::Shift], "Add to Selection").prepend_plus()]),
			]),
			MarqueeToolFsmState::Drawing => HintData(vec![HintGroup(vec![
				HintInfo::mouse(MouseMotion::Rmb, ""),
				HintInfo::keys([Key::Escape], "Cancel").prepend_slash(),
			])]),
		};
		hint_data.send_layout(responses);
	}

	fn update_cursor(&self, responses: &mut VecDeque<Message>) {
		responses.add(FrontendMessage::UpdateMouseCursor { cursor: MouseCursorIcon::Crosshair });
	}
}
