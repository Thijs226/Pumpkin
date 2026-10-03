use std::io::Write;

use pumpkin_data::packet::clientbound::play::SET_OBJECTIVE;
use pumpkin_macros::java_packet;
use pumpkin_util::{text::TextComponent, version::JavaMinecraftVersion};

use crate::{ClientPacket, NumberFormat, VarInt, WritingError, ser::NetworkWriteExt};

#[java_packet(SET_OBJECTIVE)]
pub struct CUpdateObjectives {
    pub objective_name: String,
    pub mode: u8,
    pub display_name: TextComponent,
    pub render_type: VarInt,
    pub number_format: Option<NumberFormat>,
}

impl CUpdateObjectives {
    #[must_use]
    pub const fn new(
        objective_name: String,
        mode: Mode,
        display_name: TextComponent,
        render_type: RenderType,
        number_format: Option<NumberFormat>,
    ) -> Self {
        Self {
            objective_name,
            mode: mode as u8,
            display_name,
            render_type: VarInt(render_type as i32),
            number_format,
        }
    }
}

impl ClientPacket for CUpdateObjectives {
    fn write_packet_data(
        &self,
        write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        let mut write = write;

        write.write_string(&self.objective_name)?;
        write.write_u8(self.mode)?;
        if self.mode == 0 || self.mode == 2 {
            if *version < JavaMinecraftVersion::V_1_13 {
                write.write_string(&self.display_name.clone().to_pretty_console())?;
                let render_str = if self.render_type.0 == 1 {
                    "hearts"
                } else {
                    "integer"
                };
                write.write_string(render_str)?;
            } else {
                write.write_component(&self.display_name, version)?;
                write.write_var_int(&self.render_type)?;
                if *version >= JavaMinecraftVersion::V_1_20_3 {
                    write.write_option(&self.number_format, |p, v| {
                        v.write_for_version(p, version)
                    })?;
                }
            }
        }
        Ok(())
    }
}

pub enum Mode {
    Add,
    Remove,
    Update,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderType {
    Integer,
    Hearts,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_nbt::deserializer::NbtReadHelperJava;
    use pumpkin_util::text::style::Style;
    use std::io::Cursor;

    #[test]
    fn styled_objective_preserves_explicit_decorations() -> Result<(), Box<dyn std::error::Error>> {
        let style = Style {
            bold: Some(true),
            italic: Some(false),
            ..Style::default()
        };
        let packet = CUpdateObjectives::new(
            "b".into(),
            Mode::Add,
            TextComponent::text("T"),
            RenderType::Integer,
            Some(NumberFormat::Styled(style)),
        );
        let mut bytes = Vec::new();
        packet.write_packet_data(&mut bytes, &JavaMinecraftVersion::V_26_3)?;
        assert_eq!(&bytes[..10], [1, b'b', 0, 8, 0, 1, b'T', 0, 1, 1]);
        let mut payload = Cursor::new(&bytes[10..]);
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
        assert_eq!(payload.position() as usize, bytes.len() - 10);
        Ok(())
    }
}
