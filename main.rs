use anyhow::Result;
use dogecoin::{script, ScriptBuf};
use dogecoin::opcodes::all::OP_RETURN;
use dogecoin::script::PushBytesBuf;
use integer_encoding::VarInt;

fn main() {
    println!("Hello, world!");
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scid {
    pub block_height: u32,
    pub tx_index: u32,
    pub output_index: u16,
}

impl Scid {
    pub fn new(block_height: u32, tx_index: u32, output_index: u16) -> Scid {
        assert!(block_height < 2_u32.pow(24));
        assert!(tx_index < 2_u32.pow(24));
        Scid {
            block_height,
            tx_index,
            output_index,
        }
    }

    pub fn to_u64(&self) -> u64 {
        let mut result = 0_u64;
        result |= (self.block_height as u64) << 40;
        result |= (self.tx_index as u64) << 16;
        result |= self.output_index as u64;
        result
    }

    pub fn calculate_offset(&self, other: &Scid) -> Scid {
        Scid {
            block_height: self.block_height - other.block_height,
            tx_index: self.tx_index - other.tx_index,
            output_index: self.output_index - other.output_index,
        }
    }

    // given an offset, calculate the scid
    pub fn from_offset(&self, offset: u64) -> Scid {
        let mut result = Scid::new(0, 0, 0);
        result.block_height = ((offset >> 40) & 0xFFFFFF) as u32 + self.block_height;
        result.tx_index = ((offset >> 16) & 0xFFFFFF) as u32 + self.tx_index;
        result.output_index = (offset & 0xFFFF) as u16 + self.output_index;
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
pub struct AssetTransfer {
    pub scid: Scid,
    pub target_output: u16,
    pub amount: u64, // TODO: this should probably be a u128
}

impl AssetTransfer {
    pub fn new(scid: Scid, target_output: u16, amount: u64) -> AssetTransfer {
        AssetTransfer {
            scid,
            target_output,
            amount,
        }
    }

    pub fn encode_to_tuple(&self, offset_scid: &Scid) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let adjusted_scid = if offset_scid == &self.scid { self.scid } else {self.scid.calculate_offset(offset_scid)};
        let mut scid_bytes = [0u8;8];
        adjusted_scid.to_u64().encode_var(&mut scid_bytes);
        let mut target_output_bytes = [0u8;2];
        self.target_output.encode_var(&mut target_output_bytes);
        let mut amount_bytes = [0u8;8];
        self.amount.encode_var(&mut amount_bytes);
        (Vec::from(scid_bytes), Vec::from(target_output_bytes), Vec::from(amount_bytes))
    }
}

pub fn build_transfer_script(transfers: Vec<AssetTransfer>) -> Result<ScriptBuf> {
    let mut transfer_script = script::Builder::new()
        .push_opcode(OP_RETURN)
        .push_slice(b"R");
    if let Some(first_transfer_scid) = transfers.first() {
        let first_transfer_scid = first_transfer_scid.scid;
        for transfer in transfers {
            let (scid, target_output, amount) = transfer.encode_to_tuple(&first_transfer_scid);
            transfer_script = transfer_script
                .push_slice(PushBytesBuf::try_from(scid)?)
                .push_slice(PushBytesBuf::try_from(target_output)?)
                .push_slice(PushBytesBuf::try_from(amount)?);
        }
    }
    Ok(transfer_script.into_script())
}

#[cfg(test)]
mod tests {
    use dogecoin::Address;

    #[test]
    fn test_transfer_script_building() {
        use super::*;
        let scid = Scid::new(0, 0, 0);
        let transfer = AssetTransfer::new(scid, 1, 500);
        let script = build_transfer_script(vec![transfer]).unwrap();
        println!("{:?}", script.as_script());
    }
}
