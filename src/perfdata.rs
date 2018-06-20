
//http://hg.openjdk.java.net/jdk8/jdk8/jdk/file/518d6087e01f/src/share/classes/sun/tools/jstat/resources/jstat_options

/*
typedef struct {
  jint   magic;              // magic number - 0xcafec0c0
  jbyte  byte_order;         // byte order of the buffer
  jbyte  major_version;      // major and minor version numbers
  jbyte  minor_version;
  jbyte  accessible;         // ready to access
  jint   used;               // number of PerfData memory bytes used
  jint   overflow;           // number of bytes of overflow
  jlong  mod_time_stamp;     // time stamp of last structural modification
  jint   entry_offset;       // offset of the first PerfDataEntry
  jint   num_entries;        // number of allocated PerfData entries
} PerfDataPrologue;
*/
/*
typedef struct {

  jint entry_length;      // entry length in bytes
  jint name_offset;       // offset of the data item name
  jint vector_length;     // length of the vector. If 0, then scalar
  jbyte data_type;        // type of the data item -
                          // 'B','Z','J','I','S','C','D','F','V','L','['
  jbyte flags;            // flags indicating misc attributes
  jbyte data_units;       // unit of measure for the data type
  jbyte data_variability; // variability classification of data type
  jint  data_offset;      // offset of the data item

/*
  body of PerfData memory entry is variable length

  jbyte[name_length] data_name;        // name of the data item
  jbyte[pad_length] data_pad;          // alignment of data item
  j<data_type>[data_length] data_item; // array of appropriate types.
                                       // data_length is > 1 only when the
                                       // data_type is T_ARRAY.
*/
} PerfDataEntry
*/

use std::fmt;
use std::cmp;
use std::collections::BTreeMap;
use byteorder::{BigEndian, LittleEndian, ByteOrder, ReadBytesExt};
use std::io::{Error, ErrorKind};
use std::io::Read;
use std::str::from_utf8;

const MAGIC: u32 = 0xcafec0c0;

pub struct PerfData {
    entries: BTreeMap<String, PerfDataEntry>
}


impl PerfData {
    pub fn new(f: &mut Read) -> Result<PerfData, Error> {
        let magic_from_file: u32 = f.read_u32::<BigEndian>()?;

        if magic_from_file == MAGIC {
            let byte_order: i8 = f.read_i8()?;

            let prolog;
            let entries;

            if byte_order == 0 {
                prolog = read_prologue::<BigEndian>(f)?;
                entries = read_entries::<BigEndian>(&prolog, f)?;
            } else {
                prolog = read_prologue::<LittleEndian>(f)?;
                entries = read_entries::<LittleEndian>(&prolog, f)?;
            }

            return Ok(PerfData {
                entries
            })

        } else {
            Err(Error::from(ErrorKind::InvalidData))
        }
    }

    pub fn entries(&self) -> &BTreeMap<String, PerfDataEntry> {
        &self.entries
    }

    pub fn get_val(&self, val: &str) -> i64 {
        self.entries.get(val).map(|ref val| &val.value).and_then(|val| {
            if let &PerfDataValue::Long(val) = val {
                return Some(val)
            }

            None
        }).unwrap_or(0)
    }

    pub fn get_uptime(&self)  -> i64 {
        let freq = self.get_val("sun.os.hrt.frequency");

        let app_time = self.get_val("sun.rt.applicationTime");

        return app_time / freq;
    }

    pub fn get_max_mem(&self)  -> i64 {
        //G1 appears to set both generation 0 & generation 1 to the jvm max capacity
        if let Some(ref metric) = self.entries.get("sun.gc.collector.0.name") {
            if let PerfDataValue::String(ref val) = metric.value {
                if val == "G1 incremental collections" {
                    return self.get_val("sun.gc.generation.0.maxCapacity");
                }
            }
        }

        return self.get_val("sun.gc.generation.0.maxCapacity") + self.get_val("sun.gc.generation.1.maxCapacity");;
    }

    pub fn get_used_mem(&self) -> i64 {
        let gen_0_space_0_cap = self.get_val("sun.gc.generation.0.space.0.used");
        let gen_0_space_1_cap = self.get_val("sun.gc.generation.0.space.1.used");
        let gen_0_space_2_cap = self.get_val("sun.gc.generation.0.space.2.used");
        let gen_1_space_1_cap = self.get_val("sun.gc.generation.1.space.0.used");
        let metaspace_used = self.get_val("sun.gc.metaspace.used");

        return gen_0_space_0_cap + gen_0_space_1_cap + gen_0_space_2_cap + gen_1_space_1_cap + metaspace_used;
    }

    pub fn get_full_gc(&self) -> i64 {
        let freq = self.get_val("sun.os.hrt.frequency");

        let full_gc = self.get_val("sun.gc.collector.1.time");

        return full_gc / freq;
    }

    pub fn get_total_gc(&self) -> i64 {
        let freq = self.get_val("sun.os.hrt.frequency");

        let full_gc = self.get_val("sun.gc.collector.1.time");
        let other_gc = self.get_val("sun.gc.collector.0.time");

        return (full_gc + other_gc) / freq;
    }

    pub fn get_gc_full_count(&self) -> i64 {
        return self.get_val("sun.gc.collector.1.invocations");
    }

    pub fn get_gc_count(&self) -> i64 {
        return self.get_val("sun.gc.collector.1.invocations") + self.get_val("sun.gc.collector.0.invocations")
    }
}

pub fn read_prologue<T: ByteOrder>(f: &mut Read) -> Result<PerfDataPrologue, Error> {


    let major_version = f.read_i8()?;
    let minor_version = f.read_i8()?;
    let accessible = f.read_i8()?;
    let used = f.read_i32::<T>()?;
    let overflow = f.read_i32::<T>()?;
    let mod_time_stamp = f.read_i64::<T>()?;
    let entry_offset = f.read_i32::<T>()?;
    let num_entries = f.read_i32::<T>()?;


    Ok(
    PerfDataPrologue {
        major_version,
        minor_version,
        accessible,
        used,
        overflow,
        mod_time_stamp,
        entry_offset,
        num_entries
    })
}

pub fn read_entries<T: ByteOrder>(prolog: &PerfDataPrologue, f: &mut Read) -> Result<BTreeMap<String, PerfDataEntry>,Error> {

    let mut internal_buffer = [0; 8192];

    let mut entries = BTreeMap::new();

    for _ in 0..prolog.num_entries {
        let entry_length = f.read_i32::<T>()?;
        let name_offset = f.read_i32::<T>()?;
        let _vector_length = f.read_i32::<T>()?;

        let data_type = f.read_u8()?;
        let _flags = f.read_i8()?;
        let data_units = f.read_u8()?;
        let data_variability = f.read_u8()?;
        let data_offset = f.read_i32::<T>()?;

        let name = {
            
            let name_length = (data_offset - name_offset) as usize;

            let mut sub_buffer = &mut internal_buffer[0..name_length];

            f.read_exact(sub_buffer)?;

            let string_buffer = from_utf8(sub_buffer).map_err(|_| ErrorKind::InvalidData)?;
            let null_term_index = string_buffer.find('\0').unwrap_or(name_length);

            String::from(&string_buffer[0..null_term_index])
        };


        let value = {

            let value_length = (entry_length - data_offset) as usize;

            match data_type {
                b'B'=> {

                    let mut sub_buffer = &mut internal_buffer[0..value_length];

                    f.read_exact(sub_buffer)?;

                    let string_buffer = from_utf8(sub_buffer).map_err(|_| ErrorKind::InvalidData)?;
                    let null_term_index = string_buffer.find('\0').unwrap_or(value_length);

                    PerfDataValue::String(String::from(&string_buffer[0..null_term_index]))
                },
                b'J' => {
                    PerfDataValue::Long(f.read_i64::<T>()?)
                }
                _ => {
                    print!("Data type is: {}", data_type as char);
                    unimplemented!()
                }
            }
        };


        let entry = PerfDataEntry {
            value,
            variability: PerfDataVariability::from(data_variability),
            unit: PerfDataUnit::from(data_units)
        };

        entries.insert(name, entry);
    }

    Ok(entries)
}



#[derive(Debug)]
pub struct PerfDataPrologue {
    major_version: i8,
    minor_version: i8,
    accessible: i8,
    used: i32,
    overflow: i32,
    mod_time_stamp: i64,
    entry_offset: i32,
    num_entries: i32
}

#[derive(Debug, PartialEq)]
pub enum PerfDataUnit {
    None,
    Bytes,
    Ticks,
    Events,
    String,
    Hertz,
    Other(u8)
}



#[derive(Debug)]
pub struct PerfDataEntry {
    value: PerfDataValue,
    unit: PerfDataUnit,
    variability: PerfDataVariability
}

#[derive(Debug)]
pub enum PerfDataValue {
    String(String),
    Long(i64)
}

impl fmt::Display for PerfDataValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &PerfDataValue::String(ref val) => write!(f, "{}", val),
            &PerfDataValue::Long(ref val) => write!(f, "{}", val),
        }
    }
}

impl fmt::Display for PerfDataEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.unit == PerfDataUnit::Bytes {
            if let PerfDataValue::Long(val) = self.value {
                return write!(f, "{}", convert(val as f64))
            }
        }
        write!(f, "{} ({:?})", self.value, self.unit)
    }
}



#[derive(Debug)]
enum PerfDataVariability {
    Constant,
    Monotonic,
    Variable,
}

impl From<u8> for PerfDataUnit {
    fn from(byte: u8) -> PerfDataUnit {
        match byte {
            1 => PerfDataUnit::None,
            2 => PerfDataUnit::Bytes,
            3 => PerfDataUnit::Ticks,
            4 => PerfDataUnit::Events,
            5 => PerfDataUnit::String,
            6 => PerfDataUnit::Hertz,
            other => PerfDataUnit::Other(other)
        }
    }
}

impl From<u8> for PerfDataVariability {
    fn from(byte: u8) -> PerfDataVariability {
        match byte {
            2 => PerfDataVariability::Monotonic,
            3 => PerfDataVariability::Variable,
            _ => PerfDataVariability::Constant,
        }
    }
}


pub fn convert(num: f64) -> String {
  let negative = if num.is_sign_positive() { "" } else { "-" };
  let num = num.abs();
  let units = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
  if num < 1_f64 {
      return format!("{}{} {}", negative, num, "B");
  }
  let delimiter = 1000_f64;
  let exponent = cmp::min((num.ln() / delimiter.ln()).floor() as i32, (units.len() - 1) as i32);
  let pretty_bytes = format!("{:.2}", num / delimiter.powi(exponent)).parse::<f64>().unwrap() * 1_f64;
  let unit = units[exponent as usize];
  format!("{}{} ({})", negative, pretty_bytes, unit)
}
