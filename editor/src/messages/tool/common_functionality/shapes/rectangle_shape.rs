use super::shape_utility::ShapeToolModifierKey;
use super::*;
use crate::messages::portfolio::document::graph_operation::utility_types::TransformIn;
use crate::messages::portfolio::document::node_graph::document_node_definitions::resolve_proto_node_type;
use crate::messages::portfolio::document::utility_types::document_metadata::LayerNodeIdentifier;
use crate::messages::portfolio::document::utility_types::network_interface::{InputConnector, NodeTemplate};
use crate::messages::tool::common_functionality::graph_modification_utils;
use crate::messages::tool::tool_messages::tool_prelude::*;
use glam::DAffine2;
use graph_craft::document::NodeInput;
use graph_craft::document::value::TaggedValue;
use std::collections::VecDeque;

#[derive(Default)]
pub struct Rectangle;

impl Rectangle {
	pub fn create_node() -> NodeTemplate {
		let node_type = resolve_proto_node_type(graphene_std::vector::generator_nodes::rectangle::IDENTIFIER).expect("Rectangle node can't be found");
		node_type.node_template_input_override([None, Some(NodeInput::value(TaggedValue::F64(1.), false)), Some(NodeInput::value(TaggedValue::F64(1.), false))])
	}

	pub fn update_shape(
		document: &DocumentMessageHandler,
		ipp: &InputPreprocessorMessageHandler,
		viewport: &ViewportMessageHandler,
		layer: LayerNodeIdentifier,
		shape_tool_data: &mut ShapeToolData,
		modifier: ShapeToolModifierKey,
		responses: &mut VecDeque<Message>,
	) {
		let [center, lock_ratio, _] = modifier;

		if let Some([start, end]) = shape_tool_data.data.calculate_points(document, ipp, viewport, center, lock_ratio) {
			let Some(node_id) = graph_modification_utils::get_rectangle_id(layer, &document.network_interface) else {
				return;
			};

			responses.add(NodeGraphMessage::SetInput {
				input_connector: InputConnector::node(node_id, 1),
				input: NodeInput::value(TaggedValue::F64((start.x - end.x).abs()), false),
			});
			responses.add(NodeGraphMessage::SetInput {
				input_connector: InputConnector::node(node_id, 2),
				input: NodeInput::value(TaggedValue::F64((start.y - end.y).abs()), false),
			});
			responses.add(GraphOperationMessage::TransformSet {
				layer,
				transform: DAffine2::from_translation(start.midpoint(end)),
				transform_in: TransformIn::Viewport,
				skip_rerender: false,
			});
		}
	}
}

#[cfg(test)]
mod test_rectangle {
	use crate::messages::portfolio::document::node_graph::document_node_definitions::DefinitionIdentifier;
	use crate::messages::tool::common_functionality::graph_modification_utils::NodeGraphLayer;
	use crate::test_utils::test_prelude::*;
	use glam::DAffine2;
	use graph_craft::document::value::TaggedValue;

	struct ResolvedRectangle {
		width: f64,
		height: f64,
		transform: DAffine2,
	}

	async fn get_rectangles(editor: &mut EditorTestUtils) -> Vec<ResolvedRectangle> {
		let document = editor.active_document();
		let network_interface = &document.network_interface;

		document
			.metadata()
			.all_layers()
			.filter_map(|layer| {
				let node_inputs = NodeGraphLayer::new(layer, network_interface)
					.find_node_inputs(&DefinitionIdentifier::ProtoNode(graphene_std::vector::generator_nodes::rectangle::IDENTIFIER))?;
				let Some(&TaggedValue::F64(width)) = node_inputs[1].as_value() else {
					return None;
				};
				let Some(&TaggedValue::F64(height)) = node_inputs[2].as_value() else {
					return None;
				};
				Some(ResolvedRectangle {
					width,
					height,
					transform: document.metadata().transform_to_document(layer),
				})
			})
			.collect()
	}

	#[tokio::test]
	async fn rectangle_draw_simple() {
		let mut editor = EditorTestUtils::create();
		editor.new_document().await;
		editor.drag_tool(ToolType::Rectangle, 0., 0., 40., 30., ModifierKeys::empty()).await;

		assert_eq!(editor.active_document().metadata().all_layers().count(), 1);

		let rects = get_rectangles(&mut editor).await;
		assert_eq!(rects.len(), 1);
		assert_eq!(rects[0].width, 40.);
		assert_eq!(rects[0].height, 30.);
		// Transform should place the layer at the midpoint of (0,0)→(40,30)
		assert!(rects[0].transform.abs_diff_eq(DAffine2::from_translation(DVec2::new(20., 15.)), 1e-10));
	}

	#[tokio::test]
	async fn rectangle_draw_lock_ratio() {
		let mut editor = EditorTestUtils::create();
		editor.new_document().await;
		editor.drag_tool(ToolType::Rectangle, 0., 0., 50., 30., ModifierKeys::SHIFT).await;

		let rects = get_rectangles(&mut editor).await;
		assert_eq!(rects.len(), 1);
		// With SHIFT (lock_ratio), width and height must be equal
		assert_eq!(rects[0].width, rects[0].height);
	}

	#[tokio::test]
	async fn rectangle_draw_from_center() {
		let mut editor = EditorTestUtils::create();
		editor.new_document().await;
		// ALT draws from center: drag from (50,50) to (70,60) → width=40, height=20, center=(50,50)
		editor.drag_tool(ToolType::Rectangle, 50., 50., 70., 60., ModifierKeys::ALT).await;

		let rects = get_rectangles(&mut editor).await;
		assert_eq!(rects.len(), 1);
		assert_eq!(rects[0].width, 40.);
		assert_eq!(rects[0].height, 20.);
		// Center is the drag start point (50, 50)
		assert!(rects[0].transform.abs_diff_eq(DAffine2::from_translation(DVec2::new(50., 50.)), 1e-10));
	}

	#[tokio::test]
	async fn rectangle_cancel() {
		let mut editor = EditorTestUtils::create();
		editor.new_document().await;
		editor.drag_tool_cancel_rmb(ToolType::Rectangle).await;

		let rects = get_rectangles(&mut editor).await;
		assert_eq!(rects.len(), 0);
	}
}
