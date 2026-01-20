//! Generic struct serialization/deserialization using visitor pattern.
//! Supports JSON and TOML formats through FormatWriter/FormatReader traits.

#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::borrow::Cow;

#[cfg(feature = "std")]
use std::borrow::Cow;

use vo_common_core::types::ValueKind;
use vo_common_core::runtime_type::RuntimeType;

use crate::ffi::ExternCallContext;
use crate::gc::GcRef;
use crate::objects::{interface, string as str_obj};
use crate::slot::SLOT_BYTES;
use super::tag::{get_tag_value, parse_field_options};

pub const MAX_DEPTH: usize = 64;

// ==================== Format Writer Trait ====================

/// Trait for format-specific serialization output.
pub trait FormatWriter {
    /// Called when starting a struct/object.
    fn write_object_start(&mut self);
    
    /// Called when ending a struct/object.
    fn write_object_end(&mut self);
    
    /// Called before writing a field. Returns false if field should be skipped.
    fn write_field_start(&mut self, name: &str, first: bool) -> bool;
    
    /// Called after writing a field value.
    fn write_field_end(&mut self);
    
    /// Write an integer value.
    fn write_int(&mut self, val: i64);
    
    /// Write an i32 value.
    fn write_int32(&mut self, val: i32);
    
    /// Write a float value. Returns error message if value is invalid.
    fn write_float(&mut self, val: f64) -> Result<(), &'static str>;
    
    /// Write a boolean value.
    fn write_bool(&mut self, val: bool);
    
    /// Write a string value.
    fn write_string(&mut self, val: &str);
    
    /// Write a null value.
    fn write_null(&mut self);
    
    /// Get the tag key for this format (e.g., "json", "toml").
    fn tag_key(&self) -> &'static str;
    
    /// Get the resulting bytes.
    fn into_bytes(self) -> Vec<u8>;
}

// ==================== Format Reader Trait ====================

/// Parsed value from a format.
pub enum ParsedValue<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Cow<'a, str>),
    Object(ParsedObject<'a>),
}

/// Parsed object with key-value iteration.
pub struct ParsedObject<'a> {
    pub inner: &'a str,
    pub pos: usize,
}

/// Trait for format-specific deserialization input.
pub trait FormatReader<'a>: Sized {
    /// Parse the input and return the root value.
    fn parse(input: &'a str) -> Result<ParsedValue<'a>, &'static str>;
    
    /// Parse the next key-value pair from an object.
    fn next_field(obj: &mut ParsedObject<'a>) -> Result<Option<(Cow<'a, str>, ParsedValue<'a>)>, &'static str>;
    
    /// Get the tag key for this format.
    fn tag_key() -> &'static str;
}

// ==================== Marshal Implementation ====================

/// Marshal a struct value using the given format writer.
pub fn marshal_struct_value<W: FormatWriter>(
    call: &ExternCallContext,
    ptr: GcRef,
    rttid: u32,
    writer: &mut W,
) -> Result<(), &'static str> {
    marshal_struct_value_depth(call, ptr, rttid, writer, 0)
}

fn marshal_struct_value_depth<W: FormatWriter>(
    call: &ExternCallContext,
    ptr: GcRef,
    rttid: u32,
    writer: &mut W,
    depth: usize,
) -> Result<(), &'static str> {
    if depth > MAX_DEPTH {
        return Err("max depth exceeded (possible cycle)");
    }
    
    writer.write_object_start();
    let mut first = true;
    marshal_fields_into(call, ptr, rttid, writer, &mut first, depth)?;
    writer.write_object_end();
    Ok(())
}

/// Marshal struct fields, handling embedded fields by flattening.
fn marshal_fields_into<W: FormatWriter>(
    call: &ExternCallContext,
    ptr: GcRef,
    rttid: u32,
    writer: &mut W,
    first: &mut bool,
    depth: usize,
) -> Result<(), &'static str> {
    let struct_meta_id = get_struct_meta_id(call, rttid)?;
    let struct_meta = call.struct_meta(struct_meta_id as usize).ok_or("struct meta not found")?;
    
    for field in &struct_meta.fields {
        // Skip unexported fields (lowercase first char)
        if field.name.chars().next().map(|c| c.is_lowercase()).unwrap_or(true) { continue; }
        
        let field_name = get_field_name(&field.name, field.tag.as_deref(), writer.tag_key());
        if field_name == "-" { continue; }
        
        let field_ptr = unsafe { (ptr as *const u8).add(field.offset as usize * SLOT_BYTES) };
        
        if field.embedded {
            marshal_fields_into(call, field_ptr as GcRef, field.type_info.rttid(), writer, first, depth)?;
        } else {
            if !writer.write_field_start(&field_name, *first) { continue; }
            *first = false;
            
            marshal_field_value_depth(call, ptr, field.offset as usize, field.type_info.value_kind(), field.type_info.rttid(), writer, depth)?;
            writer.write_field_end();
        }
    }
    Ok(())
}

fn marshal_field_value_depth<W: FormatWriter>(
    call: &ExternCallContext,
    struct_ptr: GcRef,
    field_offset: usize,
    vk: ValueKind,
    rttid: u32,
    writer: &mut W,
    depth: usize,
) -> Result<(), &'static str> {
    let base = struct_ptr as *const u8;
    let field_ptr = unsafe { base.add(field_offset * SLOT_BYTES) };
    
    match vk {
        ValueKind::Int | ValueKind::Int64 => {
            let val = unsafe { *(field_ptr as *const i64) };
            writer.write_int(val);
        }
        ValueKind::Int32 => {
            let val = unsafe { *(field_ptr as *const i32) };
            writer.write_int32(val);
        }
        ValueKind::Float64 => {
            let val = unsafe { *(field_ptr as *const f64) };
            writer.write_float(val)?;
        }
        ValueKind::Bool => {
            let val = unsafe { *(field_ptr as *const u8) } != 0;
            writer.write_bool(val);
        }
        ValueKind::String => {
            let str_ref = unsafe { *(field_ptr as *const u64) } as GcRef;
            if str_ref.is_null() { writer.write_string(""); }
            else { writer.write_string(str_obj::as_str(str_ref)); }
        }
        ValueKind::Struct => {
            marshal_struct_value_depth(call, field_ptr as GcRef, rttid, writer, depth + 1)?;
        }
        ValueKind::Pointer => {
            let ptr_val = unsafe { *(field_ptr as *const u64) } as GcRef;
            if ptr_val.is_null() { writer.write_null(); }
            else {
                let elem_rttid = get_pointed_type_rttid(call, rttid);
                marshal_struct_value_depth(call, ptr_val, elem_rttid, writer, depth + 1)?;
            }
        }
        ValueKind::Interface => {
            let s0 = unsafe { *(field_ptr as *const u64) };
            let s1 = unsafe { *((field_ptr as *const u64).add(1)) };
            marshal_any_value(s0, s1, writer)?;
        }
        _ => writer.write_null(),
    }
    Ok(())
}

pub fn marshal_any_value<W: FormatWriter>(slot0: u64, slot1: u64, writer: &mut W) -> Result<(), &'static str> {
    let vk = interface::unpack_value_kind(slot0);
    match vk {
        ValueKind::Void => writer.write_null(),
        ValueKind::Int | ValueKind::Int64 => writer.write_int(slot1 as i64),
        ValueKind::Float64 => {
            let val = f64::from_bits(slot1);
            writer.write_float(val)?;
        }
        ValueKind::Bool => writer.write_bool(slot1 != 0),
        ValueKind::String => {
            let str_ref = slot1 as GcRef;
            if str_ref.is_null() { writer.write_string(""); }
            else { writer.write_string(str_obj::as_str(str_ref)); }
        }
        _ => writer.write_null(),
    }
    Ok(())
}

// ==================== Unmarshal Implementation ====================

/// Unmarshal data into a struct using the given format reader.
pub fn unmarshal_struct<'a, R: FormatReader<'a>>(
    call: &mut ExternCallContext,
    ptr: GcRef,
    rttid: u32,
    input: &'a str,
) -> Result<(), &'static str> {
    let value = R::parse(input)?;
    match value {
        ParsedValue::Object(obj) => unmarshal_struct_from_object::<R>(call, ptr, rttid, obj),
        _ => Err("expected object"),
    }
}

fn unmarshal_struct_from_object<'a, R: FormatReader<'a>>(
    call: &mut ExternCallContext,
    ptr: GcRef,
    rttid: u32,
    mut obj: ParsedObject<'a>,
) -> Result<(), &'static str> {
    let struct_meta_id = get_struct_meta_id(call, rttid)?;
    
    while let Some((key, value)) = R::next_field(&mut obj)? {
        if let Some((field_ptr, fvk, field_rttid)) = find_field_by_key::<R>(call, ptr, struct_meta_id, &key)? {
            unmarshal_field_value::<R>(call, field_ptr, fvk, field_rttid, value)?;
        }
    }
    Ok(())
}

/// Find a field by key, recursively searching embedded structs.
fn find_field_by_key<'a, R: FormatReader<'a>>(
    call: &ExternCallContext,
    ptr: GcRef,
    struct_meta_id: u32,
    key: &str,
) -> Result<Option<(GcRef, ValueKind, u32)>, &'static str> {
    let struct_meta = call.struct_meta(struct_meta_id as usize).ok_or("meta not found")?;
    
    let mut embedded_fields = Vec::new();
    
    for field in &struct_meta.fields {
        let field_ptr = unsafe { (ptr as *const u8).add(field.offset as usize * SLOT_BYTES) };
        
        if field.embedded {
            embedded_fields.push((field_ptr as GcRef, field.type_info.rttid()));
        } else {
            let field_name = get_field_name(&field.name, field.tag.as_deref(), R::tag_key());
            if field_name == "-" { continue; }
            if field_name == key {
                return Ok(Some((field_ptr as GcRef, field.type_info.value_kind(), field.type_info.rttid())));
            }
        }
    }
    
    for (embed_ptr, embed_rttid) in embedded_fields {
        let embed_meta_id = get_struct_meta_id(call, embed_rttid)?;
        if let Some(result) = find_field_by_key::<R>(call, embed_ptr, embed_meta_id, key)? {
            return Ok(Some(result));
        }
    }
    
    Ok(None)
}

fn unmarshal_field_value<'a, R: FormatReader<'a>>(
    call: &mut ExternCallContext,
    field_ptr: GcRef,
    vk: ValueKind,
    rttid: u32,
    value: ParsedValue<'a>,
) -> Result<(), &'static str> {
    let field_ptr = field_ptr as *mut u8;
    match vk {
        ValueKind::Int | ValueKind::Int64 => {
            let val = match value {
                ParsedValue::Int(i) => i,
                ParsedValue::Float(f) => f as i64,
                _ => return Err("expected int"),
            };
            unsafe { *(field_ptr as *mut i64) = val; }
        }
        ValueKind::Int32 => {
            let val = match value {
                ParsedValue::Int(i) => i as i32,
                ParsedValue::Float(f) => f as i32,
                _ => return Err("expected int"),
            };
            unsafe { *(field_ptr as *mut i32) = val; }
        }
        ValueKind::Float64 => {
            let val = match value {
                ParsedValue::Int(i) => i as f64,
                ParsedValue::Float(f) => f,
                _ => return Err("expected float"),
            };
            unsafe { *(field_ptr as *mut f64) = val; }
        }
        ValueKind::Bool => {
            let val = match value {
                ParsedValue::Bool(b) => b,
                _ => return Err("expected bool"),
            };
            unsafe { *(field_ptr as *mut u8) = val as u8; }
        }
        ValueKind::String => {
            match value {
                ParsedValue::Null => { unsafe { *(field_ptr as *mut u64) = 0; } }
                ParsedValue::String(s) => {
                    let str_ref = call.alloc_str(&s);
                    unsafe { *(field_ptr as *mut u64) = str_ref as u64; }
                }
                _ => return Err("expected string"),
            }
        }
        ValueKind::Struct => {
            match value {
                ParsedValue::Null => {}
                ParsedValue::Object(obj) => {
                    unmarshal_struct_from_object::<R>(call, field_ptr as GcRef, rttid, obj)?;
                }
                _ => return Err("expected object"),
            }
        }
        ValueKind::Pointer => {
            match value {
                ParsedValue::Null => {
                    unsafe { *(field_ptr as *mut u64) = 0; }
                }
                ParsedValue::Object(obj) => {
                    let elem_rttid = get_pointed_type_rttid(call, rttid);
                    let elem_meta_id = get_struct_meta_id(call, elem_rttid)?;
                    let elem_meta = call.struct_meta(elem_meta_id as usize).ok_or("elem meta not found")?;
                    let slot_count = elem_meta.slot_count();
                    
                    let new_struct = call.gc_alloc(slot_count, &[]);
                    unmarshal_struct_from_object::<R>(call, new_struct, elem_rttid, obj)?;
                    unsafe { *(field_ptr as *mut u64) = new_struct as u64; }
                }
                _ => return Err("expected object or null"),
            }
        }
        _ => {}
    }
    Ok(())
}

// ==================== Helper Functions ====================

fn get_struct_meta_id(call: &ExternCallContext, rttid: u32) -> Result<u32, &'static str> {
    let rts = call.runtime_types();
    let rt = rts.get(rttid as usize).ok_or("type not found")?;
    match rt {
        RuntimeType::Struct { meta_id, .. } => Ok(*meta_id),
        RuntimeType::Named { struct_meta_id: Some(id), .. } => Ok(*id),
        _ => Err("not a struct type"),
    }
}

pub fn get_pointed_type_rttid(call: &ExternCallContext, ptr_rttid: u32) -> u32 {
    call.get_elem_value_rttid_from_base(ptr_rttid).rttid()
}

/// Get field name from tag or use default conversion (lowercase first char).
/// Returns "-" if field should be skipped.
pub fn get_field_name<'a>(field_name: &'a str, tag: Option<&str>, tag_key: &str) -> Cow<'a, str> {
    if let Some(tag) = tag {
        if let Some(value) = get_tag_value(tag, tag_key) {
            let (name, _omitempty) = parse_field_options(value);
            if !name.is_empty() {
                return Cow::Owned(name.to_string());
            }
        }
    }
    // Default: lowercase first char
    let mut chars = field_name.chars();
    match chars.next() {
        Some(c) if c.is_uppercase() => Cow::Owned(c.to_lowercase().collect::<String>() + chars.as_str()),
        _ => Cow::Borrowed(field_name),
    }
}
