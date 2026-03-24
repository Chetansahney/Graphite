#[derive(Debug)]
pub struct MessageData {
	name: String,
	fields: Vec<(String, usize)>,
	path: &'static str,
	line_number: usize,
}

impl MessageData {
	pub fn new(name: String, fields: Vec<(String, usize)>, path: &'static str, line_number: usize) -> MessageData {
		MessageData { name, fields, path, line_number }
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn fields(&self) -> &Vec<(String, usize)> {
		&self.fields
	}

	pub fn path(&self) -> &'static str {
		self.path
	}

	pub fn line_number(&self) -> usize {
		self.line_number
	}
}

#[derive(Debug)]
pub struct DebugMessageTree {
	name: String,
	fields: Option<Vec<String>>,
	variants: Option<Vec<DebugMessageTree>>,
	message_handler: Option<MessageData>,
	message_handler_data: Option<MessageData>,
	path: &'static str,
	line_number: usize,
}

impl DebugMessageTree {
	pub fn new(name: &str) -> DebugMessageTree {
		DebugMessageTree {
			name: name.to_string(),
			fields: None,
			variants: None,
			message_handler: None,
			message_handler_data: None,
			path: "",
			line_number: 0,
		}
	}

	pub fn add_fields(&mut self, fields: Vec<String>) {
		self.fields = Some(fields);
	}

	pub fn set_path(&mut self, path: &'static str) {
		self.path = path;
	}

	pub fn set_line_number(&mut self, line_number: usize) {
		self.line_number = line_number
	}

	pub fn add_variant(&mut self, variant: DebugMessageTree) {
		if let Some(variants) = &mut self.variants {
			variants.push(variant);
		} else {
			self.variants = Some(vec![variant]);
		}
	}

	pub fn add_message_handler_data_field(&mut self, message_handler_data: MessageData) {
		self.message_handler_data = Some(message_handler_data);
	}

	pub fn add_message_handler_field(&mut self, message_handler: MessageData) {
		self.message_handler = Some(message_handler);
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn fields(&self) -> Option<&Vec<String>> {
		self.fields.as_ref()
	}

	pub fn path(&self) -> &'static str {
		self.path
	}

	pub fn line_number(&self) -> usize {
		self.line_number
	}

	pub fn variants(&self) -> Option<&Vec<DebugMessageTree>> {
		self.variants.as_ref()
	}

	pub fn message_handler_data_fields(&self) -> Option<&MessageData> {
		self.message_handler_data.as_ref()
	}

	pub fn message_handler_fields(&self) -> Option<&MessageData> {
		self.message_handler.as_ref()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn message_data_getters_return_constructed_values() {
		let fields = vec![("field_a".to_string(), 1), ("field_b".to_string(), 2)];
		let data = MessageData::new("MyMessage".to_string(), fields.clone(), "src/foo.rs", 42);

		assert_eq!(data.name(), "MyMessage");
		assert_eq!(data.fields(), &fields);
		assert_eq!(data.path(), "src/foo.rs");
		assert_eq!(data.line_number(), 42);
	}

	#[test]
	fn debug_message_tree_new_has_empty_optional_fields() {
		let tree = DebugMessageTree::new("RootMessage");

		assert_eq!(tree.name(), "RootMessage");
		assert!(tree.fields().is_none());
		assert!(tree.variants().is_none());
		assert!(tree.message_handler_fields().is_none());
		assert!(tree.message_handler_data_fields().is_none());
		assert_eq!(tree.path(), "");
		assert_eq!(tree.line_number(), 0);
	}

	#[test]
	fn debug_message_tree_add_fields_stores_field_names() {
		let mut tree = DebugMessageTree::new("Msg");
		tree.add_fields(vec!["alpha".to_string(), "beta".to_string()]);

		let fields = tree.fields().expect("fields should be set");
		assert_eq!(fields, &vec!["alpha".to_string(), "beta".to_string()]);
	}

	#[test]
	fn debug_message_tree_set_path_and_line_number() {
		let mut tree = DebugMessageTree::new("Msg");
		tree.set_path("src/messages/mod.rs");
		tree.set_line_number(99);

		assert_eq!(tree.path(), "src/messages/mod.rs");
		assert_eq!(tree.line_number(), 99);
	}

	#[test]
	fn debug_message_tree_add_single_variant() {
		let mut root = DebugMessageTree::new("Root");
		root.add_variant(DebugMessageTree::new("VariantA"));

		let variants = root.variants().expect("variants should be set");
		assert_eq!(variants.len(), 1);
		assert_eq!(variants[0].name(), "VariantA");
	}

	#[test]
	fn debug_message_tree_add_multiple_variants() {
		let mut root = DebugMessageTree::new("Root");
		root.add_variant(DebugMessageTree::new("VariantA"));
		root.add_variant(DebugMessageTree::new("VariantB"));
		root.add_variant(DebugMessageTree::new("VariantC"));

		let variants = root.variants().expect("variants should be set");
		assert_eq!(variants.len(), 3);
		assert_eq!(variants[0].name(), "VariantA");
		assert_eq!(variants[1].name(), "VariantB");
		assert_eq!(variants[2].name(), "VariantC");
	}

	#[test]
	fn debug_message_tree_add_message_handler() {
		let mut tree = DebugMessageTree::new("Msg");
		let handler_data = MessageData::new("Handler".to_string(), vec![], "path.rs", 5);
		tree.add_message_handler_field(handler_data);

		let handler = tree.message_handler_fields().expect("handler should be set");
		assert_eq!(handler.name(), "Handler");
	}

	#[test]
	fn debug_message_tree_add_message_handler_data() {
		let mut tree = DebugMessageTree::new("Msg");
		let data = MessageData::new("HandlerData".to_string(), vec![], "data.rs", 10);
		tree.add_message_handler_data_field(data);

		let handler_data = tree.message_handler_data_fields().expect("handler data should be set");
		assert_eq!(handler_data.name(), "HandlerData");
	}
}
