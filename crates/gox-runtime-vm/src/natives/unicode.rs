//! Native implementations for the unicode package.

use gox_vm::native::{NativeCtx, NativeResult, NativeRegistry};

pub fn register(registry: &mut NativeRegistry) {
    // Classification
    registry.register("unicode.IsLetter", native_is_letter);
    registry.register("unicode.IsDigit", native_is_digit);
    registry.register("unicode.IsSpace", native_is_space);
    registry.register("unicode.IsUpper", native_is_upper);
    registry.register("unicode.IsLower", native_is_lower);
    registry.register("unicode.IsPrint", native_is_print);
    registry.register("unicode.IsGraphic", native_is_graphic);
    registry.register("unicode.IsControl", native_is_control);
    registry.register("unicode.IsPunct", native_is_punct);
    registry.register("unicode.IsSymbol", native_is_symbol);
    registry.register("unicode.IsNumber", native_is_number);
    
    // Case conversion
    registry.register("unicode.ToUpper", native_to_upper);
    registry.register("unicode.ToLower", native_to_lower);
    registry.register("unicode.ToTitle", native_to_title);
}

fn native_is_letter(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_alphabetic());
    NativeResult::Ok(1)
}

fn native_is_digit(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_ascii_digit());
    NativeResult::Ok(1)
}

fn native_is_space(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_whitespace());
    NativeResult::Ok(1)
}

fn native_is_upper(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_uppercase());
    NativeResult::Ok(1)
}

fn native_is_lower(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_lowercase());
    NativeResult::Ok(1)
}

fn native_is_print(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, !r.is_control());
    NativeResult::Ok(1)
}

fn native_is_graphic(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, !r.is_control() && !r.is_whitespace());
    NativeResult::Ok(1)
}

fn native_is_control(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_control());
    NativeResult::Ok(1)
}

fn native_is_punct(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_ascii_punctuation());
    NativeResult::Ok(1)
}

fn native_is_symbol(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    // Symbol categories: mathematical, currency, etc.
    let is_symbol = matches!(r, '$' | '+' | '<' | '=' | '>' | '^' | '`' | '|' | '~' | '¢'..='¥' | '©' | '®' | '°' | '±' | '×' | '÷');
    ctx.ret_bool(0, is_symbol);
    NativeResult::Ok(1)
}

fn native_is_number(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    ctx.ret_bool(0, r.is_numeric());
    NativeResult::Ok(1)
}

fn native_to_upper(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    let upper = r.to_uppercase().next().unwrap_or(r);
    ctx.ret_i64(0, upper as i64);
    NativeResult::Ok(1)
}

fn native_to_lower(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    let lower = r.to_lowercase().next().unwrap_or(r);
    ctx.ret_i64(0, lower as i64);
    NativeResult::Ok(1)
}

fn native_to_title(ctx: &mut NativeCtx) -> NativeResult {
    let r = char::from_u32(ctx.arg_i64(0) as u32).unwrap_or('\0');
    // Title case is same as uppercase for most characters
    let title = r.to_uppercase().next().unwrap_or(r);
    ctx.ret_i64(0, title as i64);
    NativeResult::Ok(1)
}

