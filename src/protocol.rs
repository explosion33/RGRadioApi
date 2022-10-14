pub const DATA_STREAM_SIZE: usize = 30;

pub struct RocketData {
    pub time: u32,
    pub altitude: f32,
    pub orx: f32,
    pub ory: f32,
    pub orz: f32,
    pub lat: f32,
    pub long: f32,
    pub fix: u8,
    pub quality: u8,
    pub cont1: bool,
    pub cont2: bool,
}

pub fn encode_stream(data: RocketData) -> Result<[u8; DATA_STREAM_SIZE], String> {
    let mut buf: Vec<u8> = vec![];

    buf.extend_from_slice(&data.time.to_le_bytes());
    buf.extend_from_slice(&data.altitude.to_le_bytes());
    buf.extend_from_slice(&data.orx.to_le_bytes());
    buf.extend_from_slice(&data.ory.to_le_bytes());
    buf.extend_from_slice(&data.orz.to_le_bytes());
    buf.extend_from_slice(&data.lat.to_le_bytes());
    buf.extend_from_slice(&data.long.to_le_bytes());


    let mut fix_qual: u8 = data.quality << 4;
    fix_qual += data.fix & 0b00001111;
    buf.push(fix_qual);

    let mut conts: u8 = 0;
    if data.cont1 {
        conts += 1;
    }
    if data.cont2 {
        conts += 2;
    }

    buf.push(conts);

    match buf.as_slice().try_into() {
        Ok(n) => Ok(n),
        Err(_) => Err("Error converting vec to slice".to_string()),
    }
}

pub fn decode_stream(buf: [u8; DATA_STREAM_SIZE]) -> Result<RocketData, String> {
    let time: u32 = u32::from_le_bytes(match buf[0..4].try_into(){
        Ok(n) => n,
        Err(_) => {return Err("error converting time to u32".to_string())},
    });

    let altitude: f32 = f32::from_le_bytes(match buf[4..8].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting altitude to f32".to_string())},
    });
    let orx: f32 = f32::from_le_bytes(match buf[8..12].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting orx to f32".to_string())},
    });
    let ory: f32 = f32::from_le_bytes(match buf[12..16].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting ory to f32".to_string())},
    });
    let orz: f32 = f32::from_le_bytes(match buf[16..20].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting orz to f32".to_string())},
    });
    let lat: f32 = f32::from_le_bytes(match buf[20..24].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting lat to f32".to_string())},
    });
    let long: f32 = f32::from_le_bytes(match buf[24..28].try_into() {
        Ok(n) => n,
        Err(_) => {return Err("error converting long to f32".to_string())},
    });

    // 28: 0000 0000
    //     qual fix
    let fix: u8 = buf[28] | 0b00001111;// first 4 (least significant) bits of 29
    let quality: u8 = buf[28] >> 4;// last 4 bits of 29

    // 00000   0   0
    //         2   1
    let cont1: bool = buf[29] & 1 == 1; // first (lsb) of 30
    let cont2: bool = buf[29] & 2 == 1; // second (lsb) of 30
    


    Ok(RocketData {time, altitude, orx, ory, orz, lat, long, fix, quality, cont1, cont2})
}
