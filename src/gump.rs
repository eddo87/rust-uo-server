use byteorder::{BigEndian, ByteOrder};

// ---------------------------------------------------------------------------
// GumpEntry – every visual element that can appear inside a Gump dialog
// ---------------------------------------------------------------------------

/// Represents a single layout command inside a UO Gump dialog.
///
/// Each variant maps 1-to-1 to a layout string the client understands
/// (e.g. `{ page 1 }`, `{ button 10 20 2714 2715 1 0 1 }`).
#[derive(Debug, Clone, PartialEq)]
pub enum GumpEntry {
    /// Switch to the given page. Elements after this belong to that page.
    Page(u32),

    /// A tiled background image.
    /// `(x, y, width, height, gump_id)`
    Background(i32, i32, i32, i32, i32),

    /// An alpha-blended rectangular region.
    /// `(x, y, width, height)`
    AlphaRegion(i32, i32, i32, i32),

    /// A pressable button.
    /// `(x, y, normal_id, pressed_id, is_page_button, reply_id, page_or_zero)`
    Button(i32, i32, i32, i32, bool, i32, i32),

    /// A text label drawn in a certain hue.
    /// `(x, y, hue, text_index)` – text_index references `Gump::text_lines`.
    Label(i32, i32, i32, u32),

    /// A cropped/clipped label.
    /// `(x, y, width, height, hue, text_index)`
    LabelCropped(i32, i32, i32, i32, i32, u32),

    /// A text-entry field the player can type into.
    /// `(x, y, width, height, hue, entry_id, text_index)`
    TextEntry(i32, i32, i32, i32, i32, i32, u32),

    /// Limited-length text-entry.
    /// `(x, y, width, height, hue, entry_id, text_index, max_length)`
    TextEntryLimited(i32, i32, i32, i32, i32, i32, u32, i32),

    /// A check-box (toggle).
    /// `(x, y, inactive_id, active_id, initially_checked, switch_id)`
    CheckBox(i32, i32, i32, i32, bool, i32),

    /// A radio button (mutually exclusive within a group/page).
    /// `(x, y, inactive_id, active_id, initially_selected, switch_id)`
    Radio(i32, i32, i32, i32, bool, i32),

    /// A static gump image.
    /// `(x, y, gump_id)`
    Image(i32, i32, i32),

    /// A tinted gump image.
    /// `(x, y, gump_id, hue)`
    ImageTinted(i32, i32, i32, i32),

    /// An in-game item displayed in the gump.
    /// `(x, y, item_id)`
    Item(i32, i32, i32),

    /// A tinted in-game item.
    /// `(x, y, item_id, hue)`
    ItemTinted(i32, i32, i32, i32),

    /// An HTML text area.
    /// `(x, y, width, height, text_index, has_background, has_scrollbar)`
    Html(i32, i32, i32, i32, u32, bool, bool),

    /// An XMFHTMLGUMP – client-localized text by cliloc number.
    /// `(x, y, width, height, cliloc_id, has_background, has_scrollbar)`
    HtmlLocalized(i32, i32, i32, i32, i32, bool, bool),

    /// Tooltip / hover text via cliloc number.
    /// `(cliloc_id)`
    Tooltip(i32),

    /// A group marker (used to create radio-button groups).
    /// `(group_id)`
    Group(i32),

    /// End of a group.
    EndGroup,
}

impl GumpEntry {
    /// Serialize this entry into the layout string format the UO client expects.
    pub fn to_layout_string(&self) -> String {
        match self {
            GumpEntry::Page(page) => format!("{{ page {} }}", page),
            GumpEntry::Background(x, y, w, h, gump_id) => {
                format!("{{ resizepic {} {} {} {} {} }}", x, y, gump_id, w, h)
            }
            GumpEntry::AlphaRegion(x, y, w, h) => {
                format!("{{ checkertrans {} {} {} {} }}", x, y, w, h)
            }
            GumpEntry::Button(x, y, normal, pressed, is_page, reply, page) => {
                let btn_type = if *is_page { 0 } else { 1 };
                format!(
                    "{{ button {} {} {} {} {} {} {} }}",
                    x, y, normal, pressed, btn_type, page, reply
                )
            }
            GumpEntry::Label(x, y, hue, text_idx) => {
                format!("{{ text {} {} {} {} }}", x, y, hue, text_idx)
            }
            GumpEntry::LabelCropped(x, y, w, h, hue, text_idx) => {
                format!("{{ croppedtext {} {} {} {} {} {} }}", x, y, w, h, hue, text_idx)
            }
            GumpEntry::TextEntry(x, y, w, h, hue, entry_id, text_idx) => {
                format!(
                    "{{ textentry {} {} {} {} {} {} {} }}",
                    x, y, w, h, hue, entry_id, text_idx
                )
            }
            GumpEntry::TextEntryLimited(x, y, w, h, hue, entry_id, text_idx, max_len) => {
                format!(
                    "{{ textentrylimited {} {} {} {} {} {} {} {} }}",
                    x, y, w, h, hue, entry_id, text_idx, max_len
                )
            }
            GumpEntry::CheckBox(x, y, inactive, active, checked, switch_id) => {
                let initial = if *checked { 1 } else { 0 };
                format!(
                    "{{ checkbox {} {} {} {} {} {} }}",
                    x, y, inactive, active, initial, switch_id
                )
            }
            GumpEntry::Radio(x, y, inactive, active, selected, switch_id) => {
                let initial = if *selected { 1 } else { 0 };
                format!(
                    "{{ radio {} {} {} {} {} {} }}",
                    x, y, inactive, active, initial, switch_id
                )
            }
            GumpEntry::Image(x, y, gump_id) => {
                format!("{{ gumppic {} {} {} }}", x, y, gump_id)
            }
            GumpEntry::ImageTinted(x, y, gump_id, hue) => {
                format!("{{ gumppic {} {} {} hue={} }}", x, y, gump_id, hue)
            }
            GumpEntry::Item(x, y, item_id) => {
                format!("{{ tilepic {} {} {} }}", x, y, item_id)
            }
            GumpEntry::ItemTinted(x, y, item_id, hue) => {
                format!("{{ tilepichue {} {} {} {} }}", x, y, item_id, hue)
            }
            GumpEntry::Html(x, y, w, h, text_idx, bg, scroll) => {
                let bg_val = if *bg { 1 } else { 0 };
                let scroll_val = if *scroll { 1 } else { 0 };
                format!(
                    "{{ htmlgump {} {} {} {} {} {} {} }}",
                    x, y, w, h, text_idx, bg_val, scroll_val
                )
            }
            GumpEntry::HtmlLocalized(x, y, w, h, cliloc, bg, scroll) => {
                let bg_val = if *bg { 1 } else { 0 };
                let scroll_val = if *scroll { 1 } else { 0 };
                format!(
                    "{{ xmfhtmlgump {} {} {} {} {} {} {} }}",
                    x, y, w, h, cliloc, bg_val, scroll_val
                )
            }
            GumpEntry::Tooltip(cliloc) => format!("{{ tooltip {} }}", cliloc),
            GumpEntry::Group(id) => format!("{{ group {} }}", id),
            GumpEntry::EndGroup => "{ endgroup }".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Gump – the complete dialog definition
// ---------------------------------------------------------------------------

/// A complete Gump (UI dialog) that can be serialized to packet 0xB0.
#[derive(Debug, Clone)]
pub struct Gump {
    /// Server-assigned unique identifier for this gump type.
    pub gump_id: u32,
    /// Serial of the object this gump is associated with (often 0).
    pub serial: u32,
    /// Screen x-coordinate where the gump is displayed.
    pub x: i32,
    /// Screen y-coordinate where the gump is displayed.
    pub y: i32,
    /// Whether the gump can be moved by the player.
    pub movable: bool,
    /// Whether the gump can be closed with right-click.
    pub closable: bool,
    /// Whether the gump can be disposed (via client command).
    pub disposable: bool,
    /// Whether the gump is resizable.
    pub resizable: bool,
    /// The ordered list of visual entries (layout commands).
    pub entries: Vec<GumpEntry>,
    /// Shared text lines referenced by index from Label / TextEntry / Html.
    pub text_lines: Vec<String>,
}

impl Gump {
    pub fn new(gump_id: u32, serial: u32) -> Self {
        Self {
            gump_id,
            serial,
            x: 0,
            y: 0,
            movable: true,
            closable: true,
            disposable: true,
            resizable: true,
            entries: Vec::new(),
            text_lines: Vec::new(),
        }
    }

    // -- Layout string compilation ------------------------------------------

    /// Build the full layout string (null-terminated) that goes into the 0xB0 packet.
    fn compile_layout(&self) -> String {
        let mut layout = String::new();

        if !self.movable {
            layout.push_str("{ nomove }");
        }
        if !self.closable {
            layout.push_str("{ noclose }");
        }
        if !self.disposable {
            layout.push_str("{ nodispose }");
        }
        if !self.resizable {
            layout.push_str("{ noresize }");
        }

        for entry in &self.entries {
            layout.push_str(&entry.to_layout_string());
        }

        layout
    }

    // -- Packet 0xB0 (Gump dialog) -----------------------------------------

    /// Serialize this Gump into a UO packet 0xB0 (General Gump).
    ///
    /// Wire format (big-endian):
    /// ```text
    /// Byte  Field
    /// 0     0xB0            – packet id
    /// 1-2   u16 length      – total packet length
    /// 3-6   u32 serial      – serial of associated object
    /// 7-10  u32 gump_id     – type id of this gump
    /// 11-14 u32 x           – screen x
    /// 15-18 u32 y           – screen y
    /// 19-20 u16 layout_len  – length of layout string (with null)
    /// 21..  layout string   – null-terminated ASCII
    /// ..    u16 text_count  – number of text lines
    /// for each text line:
    ///       u16 text_len    – length in UTF-16 code-units
    ///       ..  UTF-16 BE   – the text (2 bytes per code-unit)
    /// ```
    pub fn to_packet(&self) -> Vec<u8> {
        let layout = self.compile_layout();
        let layout_bytes = layout.as_bytes();
        let layout_len = layout_bytes.len() as u16 + 1; // +1 for null terminator

        // Pre-encode text lines as UTF-16 BE
        let encoded_lines: Vec<Vec<u16>> = self
            .text_lines
            .iter()
            .map(|s| s.encode_utf16().collect::<Vec<u16>>())
            .collect();

        // Calculate total packet size
        let mut size: usize = 0;
        size += 1; // packet id
        size += 2; // packet length
        size += 4; // serial
        size += 4; // gump_id
        size += 4; // x
        size += 4; // y
        size += 2; // layout_len
        size += layout_len as usize; // layout string + null
        size += 2; // text_count
        for line in &encoded_lines {
            size += 2; // text_len (code-units)
            size += line.len() * 2; // UTF-16 BE bytes
        }

        let mut buf = vec![0u8; size];
        let mut pos = 0;

        // Packet ID
        buf[pos] = 0xB0;
        pos += 1;

        // Packet length
        BigEndian::write_u16(&mut buf[pos..], size as u16);
        pos += 2;

        // Serial
        BigEndian::write_u32(&mut buf[pos..], self.serial);
        pos += 4;

        // Gump ID
        BigEndian::write_u32(&mut buf[pos..], self.gump_id);
        pos += 4;

        // X
        BigEndian::write_u32(&mut buf[pos..], self.x as u32);
        pos += 4;

        // Y
        BigEndian::write_u32(&mut buf[pos..], self.y as u32);
        pos += 4;

        // Layout string length (including null terminator)
        BigEndian::write_u16(&mut buf[pos..], layout_len);
        pos += 2;

        // Layout string bytes + null terminator
        buf[pos..pos + layout_bytes.len()].copy_from_slice(layout_bytes);
        pos += layout_bytes.len();
        buf[pos] = 0x00; // null terminator
        pos += 1;

        // Text lines count
        BigEndian::write_u16(&mut buf[pos..], encoded_lines.len() as u16);
        pos += 2;

        // Each text line
        for line in &encoded_lines {
            BigEndian::write_u16(&mut buf[pos..], line.len() as u16);
            pos += 2;
            for &code_unit in line {
                BigEndian::write_u16(&mut buf[pos..], code_unit);
                pos += 2;
            }
        }

        buf
    }
}

// ---------------------------------------------------------------------------
// GumpBuilder – fluent API for constructing Gumps
// ---------------------------------------------------------------------------

/// A fluent builder for constructing `Gump` instances.
///
/// # Example
/// ```
/// use rust_uo_server::gump::GumpBuilder;
///
/// let gump = GumpBuilder::new(0x1234, 0xABCD)
///     .position(50, 100)
///     .not_closable()
///     .page(0)
///     .background(0, 0, 400, 300, 9200)
///     .label(20, 20, 0, "Hello World")
///     .button(20, 260, 4005, 4007, false, 1, 0)
///     .build();
/// ```
pub struct GumpBuilder {
    gump: Gump,
}

impl GumpBuilder {
    pub fn new(gump_id: u32, serial: u32) -> Self {
        Self {
            gump: Gump::new(gump_id, serial),
        }
    }

    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.gump.x = x;
        self.gump.y = y;
        self
    }

    pub fn not_movable(mut self) -> Self {
        self.gump.movable = false;
        self
    }

    pub fn not_closable(mut self) -> Self {
        self.gump.closable = false;
        self
    }

    pub fn not_disposable(mut self) -> Self {
        self.gump.disposable = false;
        self
    }

    pub fn not_resizable(mut self) -> Self {
        self.gump.resizable = false;
        self
    }

    // -- Layout entries -----------------------------------------------------

    pub fn page(mut self, page: u32) -> Self {
        self.gump.entries.push(GumpEntry::Page(page));
        self
    }

    pub fn background(mut self, x: i32, y: i32, w: i32, h: i32, gump_id: i32) -> Self {
        self.gump
            .entries
            .push(GumpEntry::Background(x, y, w, h, gump_id));
        self
    }

    pub fn alpha_region(mut self, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.gump
            .entries
            .push(GumpEntry::AlphaRegion(x, y, w, h));
        self
    }

    pub fn button(
        mut self,
        x: i32,
        y: i32,
        normal_id: i32,
        pressed_id: i32,
        is_page_button: bool,
        reply_id: i32,
        page: i32,
    ) -> Self {
        self.gump.entries.push(GumpEntry::Button(
            x,
            y,
            normal_id,
            pressed_id,
            is_page_button,
            reply_id,
            page,
        ));
        self
    }

    /// Add a text label. The text is appended to the shared text-lines list and
    /// the resulting index is stored in the layout entry.
    pub fn label(mut self, x: i32, y: i32, hue: i32, text: &str) -> Self {
        let idx = self.gump.text_lines.len() as u32;
        self.gump.text_lines.push(text.to_string());
        self.gump.entries.push(GumpEntry::Label(x, y, hue, idx));
        self
    }

    pub fn label_cropped(
        mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        hue: i32,
        text: &str,
    ) -> Self {
        let idx = self.gump.text_lines.len() as u32;
        self.gump.text_lines.push(text.to_string());
        self.gump
            .entries
            .push(GumpEntry::LabelCropped(x, y, w, h, hue, idx));
        self
    }

    pub fn text_entry(
        mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        hue: i32,
        entry_id: i32,
        initial_text: &str,
    ) -> Self {
        let idx = self.gump.text_lines.len() as u32;
        self.gump.text_lines.push(initial_text.to_string());
        self.gump
            .entries
            .push(GumpEntry::TextEntry(x, y, w, h, hue, entry_id, idx));
        self
    }

    pub fn text_entry_limited(
        mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        hue: i32,
        entry_id: i32,
        initial_text: &str,
        max_length: i32,
    ) -> Self {
        let idx = self.gump.text_lines.len() as u32;
        self.gump.text_lines.push(initial_text.to_string());
        self.gump.entries.push(GumpEntry::TextEntryLimited(
            x, y, w, h, hue, entry_id, idx, max_length,
        ));
        self
    }

    pub fn checkbox(
        mut self,
        x: i32,
        y: i32,
        inactive_id: i32,
        active_id: i32,
        initially_checked: bool,
        switch_id: i32,
    ) -> Self {
        self.gump.entries.push(GumpEntry::CheckBox(
            x,
            y,
            inactive_id,
            active_id,
            initially_checked,
            switch_id,
        ));
        self
    }

    pub fn radio(
        mut self,
        x: i32,
        y: i32,
        inactive_id: i32,
        active_id: i32,
        initially_selected: bool,
        switch_id: i32,
    ) -> Self {
        self.gump.entries.push(GumpEntry::Radio(
            x,
            y,
            inactive_id,
            active_id,
            initially_selected,
            switch_id,
        ));
        self
    }

    pub fn image(mut self, x: i32, y: i32, gump_id: i32) -> Self {
        self.gump.entries.push(GumpEntry::Image(x, y, gump_id));
        self
    }

    pub fn image_tinted(mut self, x: i32, y: i32, gump_id: i32, hue: i32) -> Self {
        self.gump
            .entries
            .push(GumpEntry::ImageTinted(x, y, gump_id, hue));
        self
    }

    pub fn item(mut self, x: i32, y: i32, item_id: i32) -> Self {
        self.gump.entries.push(GumpEntry::Item(x, y, item_id));
        self
    }

    pub fn item_tinted(mut self, x: i32, y: i32, item_id: i32, hue: i32) -> Self {
        self.gump
            .entries
            .push(GumpEntry::ItemTinted(x, y, item_id, hue));
        self
    }

    pub fn html(
        mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        text: &str,
        has_background: bool,
        has_scrollbar: bool,
    ) -> Self {
        let idx = self.gump.text_lines.len() as u32;
        self.gump.text_lines.push(text.to_string());
        self.gump.entries.push(GumpEntry::Html(
            x,
            y,
            w,
            h,
            idx,
            has_background,
            has_scrollbar,
        ));
        self
    }

    pub fn html_localized(
        mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        cliloc_id: i32,
        has_background: bool,
        has_scrollbar: bool,
    ) -> Self {
        self.gump.entries.push(GumpEntry::HtmlLocalized(
            x,
            y,
            w,
            h,
            cliloc_id,
            has_background,
            has_scrollbar,
        ));
        self
    }

    pub fn tooltip(mut self, cliloc_id: i32) -> Self {
        self.gump.entries.push(GumpEntry::Tooltip(cliloc_id));
        self
    }

    pub fn group(mut self, group_id: i32) -> Self {
        self.gump.entries.push(GumpEntry::Group(group_id));
        self
    }

    pub fn end_group(mut self) -> Self {
        self.gump.entries.push(GumpEntry::EndGroup);
        self
    }

    /// Add a raw `GumpEntry` directly.
    pub fn entry(mut self, entry: GumpEntry) -> Self {
        self.gump.entries.push(entry);
        self
    }

    /// Add a raw text line and return its index (for manual entry construction).
    pub fn add_text_line(&mut self, text: &str) -> u32 {
        let idx = self.gump.text_lines.len() as u32;
        self.gump.text_lines.push(text.to_string());
        idx
    }

    /// Consume the builder and return the finished `Gump`.
    pub fn build(self) -> Gump {
        self.gump
    }
}

// ---------------------------------------------------------------------------
// GumpResponse – parsed client reply (packet 0xB1)
// ---------------------------------------------------------------------------

/// A parsed client response to a Gump dialog (packet 0xB1).
#[derive(Debug, Clone, PartialEq)]
pub struct GumpResponse {
    /// The serial of the object the gump was associated with.
    pub serial: u32,
    /// The gump type id the client is responding to.
    pub gump_id: u32,
    /// The button the player pressed (0 = closed / right-clicked).
    pub button_id: u32,
    /// IDs of checkboxes / radio buttons that were active.
    pub switches: Vec<u32>,
    /// Text entry responses keyed by entry-id, with their UTF-16-decoded text.
    pub text_entries: Vec<(u16, String)>,
}

/// Read a big-endian u16 from a byte slice, advancing the position.
fn read_u16_be(data: &[u8], pos: &mut usize) -> Option<u16> {
    if *pos + 2 > data.len() {
        return None;
    }
    let val = BigEndian::read_u16(&data[*pos..]);
    *pos += 2;
    Some(val)
}

/// Read a big-endian u32 from a byte slice, advancing the position.
fn read_u32_be(data: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > data.len() {
        return None;
    }
    let val = BigEndian::read_u32(&data[*pos..]);
    *pos += 4;
    Some(val)
}

/// Parse a Gump response from the raw payload of packet 0xB1.
///
/// Wire format (big-endian):
/// ```text
/// Byte  Field
/// 0     0xB1              – packet id
/// 1-2   u16 length        – total packet length
/// 3-6   u32 serial        – serial of associated object
/// 7-10  u32 gump_id       – type id of this gump
/// 11-14 u32 button_id     – which button was pressed (0 = close)
/// 15-18 u32 switch_count  – number of active switches
/// 19..  u32[]             – switch IDs (4 bytes each)
/// ..    u32 text_count    – number of text entries
/// for each text entry:
///       u16 entry_id      – the entry's id
///       u16 text_len      – length in UTF-16 code-units
///       ..  UTF-16 BE     – text data (2 bytes per code-unit)
/// ```
pub fn parse_gump_response(data: &[u8]) -> Option<GumpResponse> {
    if data.is_empty() || data[0] != 0xB1 {
        return None;
    }

    let mut pos: usize = 1;

    // Packet length
    let _packet_len = read_u16_be(data, &mut pos)?;

    // Serial
    let serial = read_u32_be(data, &mut pos)?;

    // Gump ID
    let gump_id = read_u32_be(data, &mut pos)?;

    // Button ID
    let button_id = read_u32_be(data, &mut pos)?;

    // Switch count
    let switch_count = read_u32_be(data, &mut pos)?;

    let mut switches = Vec::with_capacity(switch_count as usize);
    for _ in 0..switch_count {
        switches.push(read_u32_be(data, &mut pos)?);
    }

    // Text entry count
    let text_count = read_u32_be(data, &mut pos)?;

    let mut text_entries = Vec::with_capacity(text_count as usize);
    for _ in 0..text_count {
        let entry_id = read_u16_be(data, &mut pos)?;
        let text_len = read_u16_be(data, &mut pos)?; // in UTF-16 code-units

        let byte_len = text_len as usize * 2;
        if pos + byte_len > data.len() {
            return None;
        }

        let mut code_units = Vec::with_capacity(text_len as usize);
        for _ in 0..text_len {
            code_units.push(read_u16_be(data, &mut pos)?);
        }

        let text = String::from_utf16(&code_units).ok()?;
        text_entries.push((entry_id, text));
    }

    Some(GumpResponse {
        serial,
        gump_id,
        button_id,
        switches,
        text_entries,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- GumpEntry layout strings -------------------------------------------

    #[test]
    fn entry_page_layout() {
        let entry = GumpEntry::Page(2);
        assert_eq!(entry.to_layout_string(), "{ page 2 }");
    }

    #[test]
    fn entry_background_layout() {
        let entry = GumpEntry::Background(10, 20, 400, 300, 9200);
        assert_eq!(
            entry.to_layout_string(),
            "{ resizepic 10 20 9200 400 300 }"
        );
    }

    #[test]
    fn entry_button_reply_layout() {
        // A reply button (not a page button)
        let entry = GumpEntry::Button(50, 60, 4005, 4007, false, 1, 0);
        assert_eq!(
            entry.to_layout_string(),
            "{ button 50 60 4005 4007 1 0 1 }"
        );
    }

    #[test]
    fn entry_button_page_layout() {
        // A page-switch button
        let entry = GumpEntry::Button(50, 60, 4005, 4007, true, 0, 2);
        assert_eq!(
            entry.to_layout_string(),
            "{ button 50 60 4005 4007 0 2 0 }"
        );
    }

    #[test]
    fn entry_label_layout() {
        let entry = GumpEntry::Label(100, 200, 0x35, 3);
        assert_eq!(entry.to_layout_string(), "{ text 100 200 53 3 }");
    }

    #[test]
    fn entry_checkbox_layout() {
        let entry = GumpEntry::CheckBox(10, 20, 210, 211, true, 42);
        assert_eq!(
            entry.to_layout_string(),
            "{ checkbox 10 20 210 211 1 42 }"
        );
    }

    #[test]
    fn entry_radio_layout() {
        let entry = GumpEntry::Radio(10, 20, 210, 211, false, 7);
        assert_eq!(entry.to_layout_string(), "{ radio 10 20 210 211 0 7 }");
    }

    #[test]
    fn entry_image_layout() {
        let entry = GumpEntry::Image(5, 10, 1234);
        assert_eq!(entry.to_layout_string(), "{ gumppic 5 10 1234 }");
    }

    #[test]
    fn entry_image_tinted_layout() {
        let entry = GumpEntry::ImageTinted(5, 10, 1234, 33);
        assert_eq!(
            entry.to_layout_string(),
            "{ gumppic 5 10 1234 hue=33 }"
        );
    }

    #[test]
    fn entry_item_layout() {
        let entry = GumpEntry::Item(30, 40, 0x0E21);
        assert_eq!(entry.to_layout_string(), "{ tilepic 30 40 3617 }");
    }

    #[test]
    fn entry_item_tinted_layout() {
        let entry = GumpEntry::ItemTinted(30, 40, 0x0E21, 55);
        assert_eq!(
            entry.to_layout_string(),
            "{ tilepichue 30 40 3617 55 }"
        );
    }

    #[test]
    fn entry_html_layout() {
        let entry = GumpEntry::Html(10, 20, 300, 200, 0, true, false);
        assert_eq!(
            entry.to_layout_string(),
            "{ htmlgump 10 20 300 200 0 1 0 }"
        );
    }

    #[test]
    fn entry_html_localized_layout() {
        let entry = GumpEntry::HtmlLocalized(10, 20, 300, 200, 1060847, false, true);
        assert_eq!(
            entry.to_layout_string(),
            "{ xmfhtmlgump 10 20 300 200 1060847 0 1 }"
        );
    }

    #[test]
    fn entry_text_entry_layout() {
        let entry = GumpEntry::TextEntry(10, 20, 200, 30, 0, 5, 2);
        assert_eq!(
            entry.to_layout_string(),
            "{ textentry 10 20 200 30 0 5 2 }"
        );
    }

    #[test]
    fn entry_text_entry_limited_layout() {
        let entry = GumpEntry::TextEntryLimited(10, 20, 200, 30, 0, 5, 2, 100);
        assert_eq!(
            entry.to_layout_string(),
            "{ textentrylimited 10 20 200 30 0 5 2 100 }"
        );
    }

    #[test]
    fn entry_label_cropped_layout() {
        let entry = GumpEntry::LabelCropped(10, 20, 300, 25, 0, 1);
        assert_eq!(
            entry.to_layout_string(),
            "{ croppedtext 10 20 300 25 0 1 }"
        );
    }

    #[test]
    fn entry_alpha_region_layout() {
        let entry = GumpEntry::AlphaRegion(0, 0, 400, 300);
        assert_eq!(
            entry.to_layout_string(),
            "{ checkertrans 0 0 400 300 }"
        );
    }

    #[test]
    fn entry_tooltip_layout() {
        let entry = GumpEntry::Tooltip(1060847);
        assert_eq!(entry.to_layout_string(), "{ tooltip 1060847 }");
    }

    #[test]
    fn entry_group_layout() {
        let entry = GumpEntry::Group(1);
        assert_eq!(entry.to_layout_string(), "{ group 1 }");
    }

    #[test]
    fn entry_end_group_layout() {
        let entry = GumpEntry::EndGroup;
        assert_eq!(entry.to_layout_string(), "{ endgroup }");
    }

    // -- Gump builder -------------------------------------------------------

    #[test]
    fn builder_creates_gump_with_defaults() {
        let gump = GumpBuilder::new(0x1234, 0xABCD).build();
        assert_eq!(gump.gump_id, 0x1234);
        assert_eq!(gump.serial, 0xABCD);
        assert_eq!(gump.x, 0);
        assert_eq!(gump.y, 0);
        assert!(gump.movable);
        assert!(gump.closable);
        assert!(gump.disposable);
        assert!(gump.resizable);
        assert!(gump.entries.is_empty());
        assert!(gump.text_lines.is_empty());
    }

    #[test]
    fn builder_fluent_api() {
        let gump = GumpBuilder::new(1, 2)
            .position(50, 100)
            .not_movable()
            .not_closable()
            .page(0)
            .background(0, 0, 400, 300, 9200)
            .label(20, 20, 0, "Hello World")
            .checkbox(20, 50, 210, 211, false, 10)
            .radio(20, 80, 208, 209, true, 20)
            .text_entry(20, 110, 200, 30, 0, 1, "default")
            .button(20, 260, 4005, 4007, false, 1, 0)
            .build();

        assert_eq!(gump.x, 50);
        assert_eq!(gump.y, 100);
        assert!(!gump.movable);
        assert!(!gump.closable);
        assert_eq!(gump.entries.len(), 7);
        assert_eq!(gump.text_lines.len(), 2); // "Hello World" and "default"
        assert_eq!(gump.text_lines[0], "Hello World");
        assert_eq!(gump.text_lines[1], "default");
    }

    #[test]
    fn builder_label_auto_indexes_text() {
        let gump = GumpBuilder::new(1, 0)
            .label(0, 0, 0, "first")
            .label(0, 20, 0, "second")
            .label(0, 40, 0, "third")
            .build();

        assert_eq!(gump.text_lines, vec!["first", "second", "third"]);
        assert_eq!(gump.entries[0], GumpEntry::Label(0, 0, 0, 0));
        assert_eq!(gump.entries[1], GumpEntry::Label(0, 20, 0, 1));
        assert_eq!(gump.entries[2], GumpEntry::Label(0, 40, 0, 2));
    }

    #[test]
    fn builder_html_entry() {
        let gump = GumpBuilder::new(1, 0)
            .html(10, 20, 300, 200, "<b>Bold</b>", true, false)
            .build();

        assert_eq!(gump.text_lines, vec!["<b>Bold</b>"]);
        assert_eq!(
            gump.entries[0],
            GumpEntry::Html(10, 20, 300, 200, 0, true, false)
        );
    }

    #[test]
    fn builder_add_text_line_returns_index() {
        let mut builder = GumpBuilder::new(1, 0);
        let idx0 = builder.add_text_line("zero");
        let idx1 = builder.add_text_line("one");
        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
    }

    // -- Packet 0xB0 serialization ------------------------------------------

    #[test]
    fn to_packet_header_structure() {
        let gump = GumpBuilder::new(0x1234, 0xABCD)
            .position(100, 200)
            .build();

        let pkt = gump.to_packet();

        // Packet ID
        assert_eq!(pkt[0], 0xB0);

        // Packet length matches actual length
        let pkt_len = BigEndian::read_u16(&pkt[1..3]);
        assert_eq!(pkt_len as usize, pkt.len());

        // Serial
        assert_eq!(BigEndian::read_u32(&pkt[3..7]), 0xABCD);

        // Gump ID
        assert_eq!(BigEndian::read_u32(&pkt[7..11]), 0x1234);

        // X
        assert_eq!(BigEndian::read_u32(&pkt[11..15]), 100);

        // Y
        assert_eq!(BigEndian::read_u32(&pkt[15..19]), 200);
    }

    #[test]
    fn to_packet_layout_section() {
        let gump = GumpBuilder::new(1, 0)
            .page(0)
            .background(0, 0, 400, 300, 9200)
            .build();

        let pkt = gump.to_packet();

        // Layout length at offset 19
        let layout_len = BigEndian::read_u16(&pkt[19..21]) as usize;

        // Extract layout bytes (after 21, before null)
        let layout_start = 21;
        let layout_end = layout_start + layout_len - 1; // minus null
        let layout_str =
            std::str::from_utf8(&pkt[layout_start..layout_end]).expect("valid ASCII layout");

        assert!(layout_str.contains("{ page 0 }"));
        assert!(layout_str.contains("{ resizepic 0 0 9200 400 300 }"));

        // Null terminator
        assert_eq!(pkt[layout_end], 0x00);
    }

    #[test]
    fn to_packet_flags_in_layout() {
        let gump = GumpBuilder::new(1, 0)
            .not_movable()
            .not_closable()
            .not_disposable()
            .not_resizable()
            .build();

        let pkt = gump.to_packet();
        let layout_len = BigEndian::read_u16(&pkt[19..21]) as usize;
        let layout_str =
            std::str::from_utf8(&pkt[21..21 + layout_len - 1]).expect("valid ASCII");

        assert!(layout_str.contains("{ nomove }"));
        assert!(layout_str.contains("{ noclose }"));
        assert!(layout_str.contains("{ nodispose }"));
        assert!(layout_str.contains("{ noresize }"));
    }

    #[test]
    fn to_packet_text_lines_utf16() {
        let gump = GumpBuilder::new(1, 0)
            .label(0, 0, 0, "Hi")
            .build();

        let pkt = gump.to_packet();

        // Skip past layout section
        let layout_len = BigEndian::read_u16(&pkt[19..21]) as usize;
        let text_section_start = 21 + layout_len;

        // Text count
        let text_count = BigEndian::read_u16(&pkt[text_section_start..]);
        assert_eq!(text_count, 1);

        // First text line: length in code-units
        let text_len =
            BigEndian::read_u16(&pkt[text_section_start + 2..]) as usize;
        assert_eq!(text_len, 2); // "Hi" = 2 code-units

        // UTF-16 BE encoding of "Hi"
        let h = BigEndian::read_u16(&pkt[text_section_start + 4..]);
        let i = BigEndian::read_u16(&pkt[text_section_start + 6..]);
        assert_eq!(h, 0x0048); // 'H'
        assert_eq!(i, 0x0069); // 'i'
    }

    #[test]
    fn to_packet_unicode_text_line() {
        // Test non-ASCII characters (e.g., emoji or CJK)
        let gump = GumpBuilder::new(1, 0)
            .label(0, 0, 0, "\u{00E9}l\u{00E8}ve") // "eleve" with accents
            .build();

        let pkt = gump.to_packet();
        let layout_len = BigEndian::read_u16(&pkt[19..21]) as usize;
        let text_section_start = 21 + layout_len;

        let text_count = BigEndian::read_u16(&pkt[text_section_start..]);
        assert_eq!(text_count, 1);

        let text_len =
            BigEndian::read_u16(&pkt[text_section_start + 2..]) as usize;
        assert_eq!(text_len, 5); // 5 UTF-16 code-units

        // Decode UTF-16 back
        let mut code_units = Vec::new();
        for i in 0..text_len {
            let offset = text_section_start + 4 + i * 2;
            code_units.push(BigEndian::read_u16(&pkt[offset..]));
        }
        let decoded = String::from_utf16(&code_units).unwrap();
        assert_eq!(decoded, "\u{00E9}l\u{00E8}ve");
    }

    #[test]
    fn to_packet_multiple_text_lines() {
        let gump = GumpBuilder::new(1, 0)
            .label(0, 0, 0, "AB")
            .label(0, 20, 0, "CD")
            .build();

        let pkt = gump.to_packet();
        let layout_len = BigEndian::read_u16(&pkt[19..21]) as usize;
        let text_start = 21 + layout_len;

        let text_count = BigEndian::read_u16(&pkt[text_start..]);
        assert_eq!(text_count, 2);

        // First line: "AB" (2 code-units = 4 bytes)
        let len1 = BigEndian::read_u16(&pkt[text_start + 2..]) as usize;
        assert_eq!(len1, 2);
        let a = BigEndian::read_u16(&pkt[text_start + 4..]);
        let b = BigEndian::read_u16(&pkt[text_start + 6..]);
        assert_eq!(a, 0x0041);
        assert_eq!(b, 0x0042);

        // Second line: "CD" starts at text_start + 2 + 2 + 4 = text_start + 8
        let offset2 = text_start + 8;
        let len2 = BigEndian::read_u16(&pkt[offset2..]) as usize;
        assert_eq!(len2, 2);
        let c = BigEndian::read_u16(&pkt[offset2 + 2..]);
        let d = BigEndian::read_u16(&pkt[offset2 + 4..]);
        assert_eq!(c, 0x0043);
        assert_eq!(d, 0x0044);
    }

    #[test]
    fn to_packet_empty_gump() {
        let gump = Gump::new(0, 0);
        let pkt = gump.to_packet();

        assert_eq!(pkt[0], 0xB0);
        let total_len = BigEndian::read_u16(&pkt[1..3]) as usize;
        assert_eq!(total_len, pkt.len());

        // Layout should be just a null byte (length = 1)
        let layout_len = BigEndian::read_u16(&pkt[19..21]);
        assert_eq!(layout_len, 1);
        assert_eq!(pkt[21], 0x00); // null terminator only

        // Zero text lines
        let text_count = BigEndian::read_u16(&pkt[22..24]);
        assert_eq!(text_count, 0);

        // Total: 1 + 2 + 4 + 4 + 4 + 4 + 2 + 1 + 2 = 24
        assert_eq!(pkt.len(), 24);
    }

    // -- Packet 0xB1 parsing (GumpResponse) ---------------------------------

    #[test]
    fn parse_gump_response_close() {
        // Minimal 0xB1: player closed gump (button = 0, no switches, no text)
        let mut pkt = vec![0xB1]; // packet id

        let total_len: u16 = 23; // 1 + 2 + 4 + 4 + 4 + 4 + 4 = 23
        pkt.extend_from_slice(&total_len.to_be_bytes());

        let serial: u32 = 0x00001234;
        pkt.extend_from_slice(&serial.to_be_bytes());

        let gump_id: u32 = 0x00005678;
        pkt.extend_from_slice(&gump_id.to_be_bytes());

        let button_id: u32 = 0; // close
        pkt.extend_from_slice(&button_id.to_be_bytes());

        let switch_count: u32 = 0;
        pkt.extend_from_slice(&switch_count.to_be_bytes());

        let text_count: u32 = 0;
        pkt.extend_from_slice(&text_count.to_be_bytes());

        let resp = parse_gump_response(&pkt).unwrap();
        assert_eq!(resp.serial, 0x1234);
        assert_eq!(resp.gump_id, 0x5678);
        assert_eq!(resp.button_id, 0);
        assert!(resp.switches.is_empty());
        assert!(resp.text_entries.is_empty());
    }

    #[test]
    fn parse_gump_response_with_button() {
        let mut pkt = vec![0xB1];
        let total_len: u16 = 23;
        pkt.extend_from_slice(&total_len.to_be_bytes());
        pkt.extend_from_slice(&1u32.to_be_bytes()); // serial
        pkt.extend_from_slice(&2u32.to_be_bytes()); // gump_id
        pkt.extend_from_slice(&42u32.to_be_bytes()); // button_id
        pkt.extend_from_slice(&0u32.to_be_bytes()); // switch_count
        pkt.extend_from_slice(&0u32.to_be_bytes()); // text_count

        let resp = parse_gump_response(&pkt).unwrap();
        assert_eq!(resp.button_id, 42);
    }

    #[test]
    fn parse_gump_response_with_switches() {
        let mut pkt = vec![0xB1];

        // We'll compute the real length after building
        let placeholder_len_pos = pkt.len();
        pkt.extend_from_slice(&0u16.to_be_bytes()); // placeholder

        pkt.extend_from_slice(&0x10u32.to_be_bytes()); // serial
        pkt.extend_from_slice(&0x20u32.to_be_bytes()); // gump_id
        pkt.extend_from_slice(&1u32.to_be_bytes()); // button_id

        pkt.extend_from_slice(&3u32.to_be_bytes()); // switch_count
        pkt.extend_from_slice(&10u32.to_be_bytes()); // switch 10
        pkt.extend_from_slice(&20u32.to_be_bytes()); // switch 20
        pkt.extend_from_slice(&30u32.to_be_bytes()); // switch 30

        pkt.extend_from_slice(&0u32.to_be_bytes()); // text_count

        // Patch length
        let total = pkt.len() as u16;
        BigEndian::write_u16(&mut pkt[placeholder_len_pos..], total);

        let resp = parse_gump_response(&pkt).unwrap();
        assert_eq!(resp.switches, vec![10, 20, 30]);
    }

    #[test]
    fn parse_gump_response_with_text_entries() {
        let mut pkt = vec![0xB1];
        let placeholder_len_pos = pkt.len();
        pkt.extend_from_slice(&0u16.to_be_bytes()); // placeholder

        pkt.extend_from_slice(&0xAAu32.to_be_bytes()); // serial
        pkt.extend_from_slice(&0xBBu32.to_be_bytes()); // gump_id
        pkt.extend_from_slice(&5u32.to_be_bytes()); // button_id
        pkt.extend_from_slice(&0u32.to_be_bytes()); // switch_count

        pkt.extend_from_slice(&2u32.to_be_bytes()); // text_count

        // Entry 0: id=1, text="Hi" (UTF-16 BE)
        pkt.extend_from_slice(&1u16.to_be_bytes()); // entry_id
        pkt.extend_from_slice(&2u16.to_be_bytes()); // text_len (code-units)
        pkt.extend_from_slice(&0x0048u16.to_be_bytes()); // 'H'
        pkt.extend_from_slice(&0x0069u16.to_be_bytes()); // 'i'

        // Entry 1: id=3, text="" (empty)
        pkt.extend_from_slice(&3u16.to_be_bytes()); // entry_id
        pkt.extend_from_slice(&0u16.to_be_bytes()); // text_len

        // Patch length
        let total = pkt.len() as u16;
        BigEndian::write_u16(&mut pkt[placeholder_len_pos..], total);

        let resp = parse_gump_response(&pkt).unwrap();
        assert_eq!(resp.button_id, 5);
        assert_eq!(resp.text_entries.len(), 2);
        assert_eq!(resp.text_entries[0], (1, "Hi".to_string()));
        assert_eq!(resp.text_entries[1], (3, String::new()));
    }

    #[test]
    fn parse_gump_response_utf16_unicode() {
        let mut pkt = vec![0xB1];
        let placeholder_len_pos = pkt.len();
        pkt.extend_from_slice(&0u16.to_be_bytes());

        pkt.extend_from_slice(&0u32.to_be_bytes()); // serial
        pkt.extend_from_slice(&0u32.to_be_bytes()); // gump_id
        pkt.extend_from_slice(&1u32.to_be_bytes()); // button_id
        pkt.extend_from_slice(&0u32.to_be_bytes()); // switch_count

        pkt.extend_from_slice(&1u32.to_be_bytes()); // text_count

        // Entry: id=0, text = "cafe" with accented e (U+00E9)
        let text = "caf\u{00E9}";
        let utf16: Vec<u16> = text.encode_utf16().collect();

        pkt.extend_from_slice(&0u16.to_be_bytes()); // entry_id
        pkt.extend_from_slice(&(utf16.len() as u16).to_be_bytes());
        for cu in &utf16 {
            pkt.extend_from_slice(&cu.to_be_bytes());
        }

        let total = pkt.len() as u16;
        BigEndian::write_u16(&mut pkt[placeholder_len_pos..], total);

        let resp = parse_gump_response(&pkt).unwrap();
        assert_eq!(resp.text_entries[0].1, "caf\u{00E9}");
    }

    #[test]
    fn parse_gump_response_invalid_packet_id() {
        let pkt = vec![0xB0, 0x00, 0x03]; // wrong packet ID
        assert!(parse_gump_response(&pkt).is_none());
    }

    #[test]
    fn parse_gump_response_empty_data() {
        assert!(parse_gump_response(&[]).is_none());
    }

    #[test]
    fn parse_gump_response_truncated_data() {
        // Only packet id and partial length
        let pkt = vec![0xB1, 0x00];
        assert!(parse_gump_response(&pkt).is_none());
    }

    #[test]
    fn parse_gump_response_switches_and_text() {
        // Combined: 2 switches + 1 text entry
        let mut pkt = vec![0xB1];
        let placeholder_len_pos = pkt.len();
        pkt.extend_from_slice(&0u16.to_be_bytes());

        pkt.extend_from_slice(&0x100u32.to_be_bytes()); // serial
        pkt.extend_from_slice(&0x200u32.to_be_bytes()); // gump_id
        pkt.extend_from_slice(&7u32.to_be_bytes()); // button_id

        pkt.extend_from_slice(&2u32.to_be_bytes()); // switch_count
        pkt.extend_from_slice(&100u32.to_be_bytes());
        pkt.extend_from_slice(&200u32.to_be_bytes());

        pkt.extend_from_slice(&1u32.to_be_bytes()); // text_count
        pkt.extend_from_slice(&5u16.to_be_bytes()); // entry_id
        // "OK" in UTF-16
        pkt.extend_from_slice(&2u16.to_be_bytes());
        pkt.extend_from_slice(&0x004Fu16.to_be_bytes()); // 'O'
        pkt.extend_from_slice(&0x004Bu16.to_be_bytes()); // 'K'

        let total = pkt.len() as u16;
        BigEndian::write_u16(&mut pkt[placeholder_len_pos..], total);

        let resp = parse_gump_response(&pkt).unwrap();
        assert_eq!(resp.serial, 0x100);
        assert_eq!(resp.gump_id, 0x200);
        assert_eq!(resp.button_id, 7);
        assert_eq!(resp.switches, vec![100, 200]);
        assert_eq!(resp.text_entries, vec![(5, "OK".to_string())]);
    }

    // -- Round-trip: build gump -> serialize -> verify ----------------------

    #[test]
    fn round_trip_realistic_gump() {
        let gump = GumpBuilder::new(0xDEAD, 0xBEEF)
            .position(100, 50)
            .not_closable()
            .page(0)
            .background(0, 0, 400, 350, 9200)
            .label(20, 20, 0x35, "Welcome!")
            .checkbox(20, 60, 210, 211, true, 1)
            .radio(20, 90, 208, 209, false, 2)
            .text_entry(20, 130, 200, 30, 0, 10, "Type here")
            .html(20, 170, 360, 100, "<b>Info</b>", true, true)
            .item(300, 20, 0x0E21)
            .button(150, 310, 4005, 4007, false, 100, 0)
            .build();

        let pkt = gump.to_packet();

        // Verify structural integrity
        assert_eq!(pkt[0], 0xB0);
        let total_len = BigEndian::read_u16(&pkt[1..3]) as usize;
        assert_eq!(total_len, pkt.len());

        assert_eq!(BigEndian::read_u32(&pkt[3..7]), 0xBEEF);
        assert_eq!(BigEndian::read_u32(&pkt[7..11]), 0xDEAD);
        assert_eq!(BigEndian::read_u32(&pkt[11..15]), 100);
        assert_eq!(BigEndian::read_u32(&pkt[15..19]), 50);

        // Layout string should contain all entries
        let layout_len = BigEndian::read_u16(&pkt[19..21]) as usize;
        let layout =
            std::str::from_utf8(&pkt[21..21 + layout_len - 1]).unwrap();

        assert!(layout.contains("{ noclose }"));
        assert!(layout.contains("{ page 0 }"));
        assert!(layout.contains("{ resizepic 0 0 9200 400 350 }"));
        assert!(layout.contains("{ text 20 20 53 0 }"));
        assert!(layout.contains("{ checkbox 20 60 210 211 1 1 }"));
        assert!(layout.contains("{ radio 20 90 208 209 0 2 }"));
        assert!(layout.contains("{ textentry 20 130 200 30 0 10 1 }"));
        assert!(layout.contains("{ htmlgump 20 170 360 100 2 1 1 }"));
        assert!(layout.contains("{ tilepic 300 20 3617 }"));
        assert!(layout.contains("{ button 150 310 4005 4007 1 0 100 }"));

        // Text lines section
        let text_start = 21 + layout_len;
        let text_count = BigEndian::read_u16(&pkt[text_start..]);
        assert_eq!(text_count, 3); // "Welcome!", "Type here", "<b>Info</b>"

        // We trust UTF-16 encoding from prior tests; just check count
    }
}
