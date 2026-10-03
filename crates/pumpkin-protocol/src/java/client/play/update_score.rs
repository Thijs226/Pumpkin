use pumpkin_data::packet::clientbound::play::SET_SCORE;
use pumpkin_util::text::TextComponent;

use pumpkin_macros::java_packet;

use crate::{
    ClientPacket, NumberFormat, VarInt,
    ser::{NetworkWriteExt, WritingError},
};

/// Sent by the server to create or update a score for an entity on a specific objective.
///
/// This packet is the primary way to manage scoreboard data. In the latest protocol,
/// it also supports optional custom formatting for how the numeric score is displayed.
#[java_packet(SET_SCORE)]
pub struct CUpdateScore {
    /// The name of the entity whose score is being updated (e.g., a player's username
    /// or a non-player entry like "Kills").
    pub entity_name: String,
    /// The internal name of the objective this score belongs to.
    pub objective_name: String,
    /// The actual integer value of the score.
    pub value: VarInt,
    /// An optional custom name for the entity to be displayed in the scoreboard.
    /// If `None`, the `entity_name` is used by default.
    pub display_name: Option<TextComponent>,
    /// Optional formatting for the number (e.g., blank, fixed text, or styled).
    /// This allows for scores to appear as something other than raw numbers.
    pub number_format: Option<NumberFormat>,
}

impl CUpdateScore {
    #[must_use]
    pub const fn new(
        entity_name: String,
        objective_name: String,
        value: VarInt,
        display_name: Option<TextComponent>,
        number_format: Option<NumberFormat>,
    ) -> Self {
        Self {
            entity_name,
            objective_name,
            value,
            display_name,
            number_format,
        }
    }

    #[must_use]
    pub const fn new_remove(entity_name: String, objective_name: String) -> Self {
        Self {
            entity_name,
            objective_name,
            value: VarInt(0),
            display_name: None,
            number_format: None,
        }
    }
}

impl ClientPacket for CUpdateScore {
    fn write_packet_data(
        &self,
        mut write: impl std::io::Write,
        version: &pumpkin_util::version::JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_string(&self.entity_name)?;
        if *version >= pumpkin_util::version::JavaMinecraftVersion::V_1_20_3 {
            write.write_string(&self.objective_name)?;
            write.write_var_int(&self.value)?;
            write.write_option(&self.display_name, |w, t| w.write_component(t, version))?;
            write.write_option(&self.number_format, |w, n| n.write_for_version(w, version))
        } else if *version <= pumpkin_util::version::JavaMinecraftVersion::V_1_7_6 {
            write.write_u8(0)?;
            write.write_string(&self.objective_name)?;
            write.write_i32_be(self.value.0)
        } else {
            write.write_u8(0)?;
            write.write_string(&self.objective_name)?;
            write.write_var_int(&self.value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_nbt::deserializer::NbtReadHelperJava;
    use pumpkin_util::text::style::Style;
    use pumpkin_util::version::JavaMinecraftVersion;
    use std::io::Cursor;

    #[test]
    fn styled_score_writes_style_payload() -> Result<(), Box<dyn std::error::Error>> {
        let style = Style {
            bold: Some(true),
            italic: Some(false),
            ..Style::default()
        };
        let packet = CUpdateScore::new(
            "A".into(),
            "b".into(),
            VarInt(7),
            None,
            Some(NumberFormat::Styled(style)),
        );
        let mut bytes = Vec::new();
        packet.write_packet_data(&mut bytes, &JavaMinecraftVersion::V_26_3)?;
        assert_eq!(&bytes[..8], [1, b'A', 1, b'b', 7, 0, 1, 1]);
        let mut payload = Cursor::new(&bytes[8..]);
        let actual = pumpkin_nbt::Nbt::read_unnamed(&mut NbtReadHelperJava::new(&mut payload))?;
        let mut expected_bytes = Cursor::new(
            &[
                10, 1, 0, 4, b'b', b'o', b'l', b'd', 1, 1, 0, 6, b'i', b't', b'a', b'l', b'i',
                b'c', 0, 0,
            ][..],
        );
        let expected =
            pumpkin_nbt::Nbt::read_unnamed(&mut NbtReadHelperJava::new(&mut expected_bytes))?;
        assert_eq!(actual.root_tag, expected.root_tag);
        assert_eq!(payload.position() as usize, bytes.len() - 8);
        Ok(())
    }

    #[test]
    fn empty_styled_score_writes_empty_compound() -> Result<(), WritingError> {
        let packet = CUpdateScore::new(
            "A".into(),
            "b".into(),
            VarInt(7),
            None,
            Some(NumberFormat::Styled(Style::default())),
        );
        let mut bytes = Vec::new();
        packet.write_packet_data(&mut bytes, &JavaMinecraftVersion::V_26_3)?;
        assert_eq!(bytes, [1, b'A', 1, b'b', 7, 0, 1, 1, 10, 0]);
        Ok(())
    }

    #[test]
    fn update_score_serialization() {
        let packet = CUpdateScore::new("Alex".into(), "kills".into(), VarInt(42), None, None);

        // Modern 1.20.3+
        let mut buf_modern = Vec::new();
        packet
            .write_packet_data(&mut buf_modern, &JavaMinecraftVersion::V_1_20_3)
            .unwrap();

        // 1.8 - 1.20.2
        let mut buf_legacy = Vec::new();
        packet
            .write_packet_data(&mut buf_legacy, &JavaMinecraftVersion::V_1_8)
            .unwrap();

        // 1.7.6
        let mut buf_v1_7 = Vec::new();
        packet
            .write_packet_data(&mut buf_v1_7, &JavaMinecraftVersion::V_1_7_6)
            .unwrap();

        assert!(!buf_modern.is_empty());
        assert!(!buf_legacy.is_empty());
        assert!(!buf_v1_7.is_empty());
    }
}
