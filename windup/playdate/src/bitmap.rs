use alloc::format;

use crate::capi_state::CApiState;
use crate::color::Color;
use crate::ctypes::*;
use crate::null_terminated::ToNullTerminatedString;
use crate::Error;

/// A bitmap image.
///
/// The bitmap can be cloned which will make a clone of the pixels as well. The bitmap's pixels data
/// is freed when the bitmap is dropped.
///
/// An `Bitmap` is borrowed as an `&BitmapRef` and all methods of that type are available for
/// `Bitmap as well.
#[derive(Debug)]
pub struct Bitmap {
  /// While BitmapRef is a non-owning pointer, the Bitmap will act as the owner of the bitmap
  /// found within.
  owned: BitmapRef,
}
impl Bitmap {
  /// Allocates and returns a new `width` by `height` `Bitmap` filled with `bg_color`.
  pub fn new<'a, C>(width: i32, height: i32, bg_color: C) -> Bitmap
  where
    Color<'a>: From<C>,
  {
    // FIXME: for some reason, patterns don't appear to work here, but do work with a C example.
    let bitmap_ptr = unsafe {
      CApiState::get().cgraphics.newBitmap.unwrap()(
        width,
        height,
        Color::<'a>::from(bg_color).to_c_color(),
      )
    };
    Bitmap::from_owned_ptr(bitmap_ptr)
  }

  /// Returns a new, rotated and scaled Bitmap based on the given `bitmap`.
  pub fn from_bitmap_with_rotation(
    bitmap: &BitmapRef,
    rotation: f32,
    xscale: f32,
    yscale: f32,
  ) -> Bitmap {
    // This function could grow the bitmap by rotating and so it (conveniently?) also returns the
    // alloced size of the new bitmap.  You can get this off the bitmap data more or less if needed.
    let mut _alloced_size: i32 = 0;
    let bitmap_ptr = unsafe {
      CApiState::get().cgraphics.rotatedBitmap.unwrap()(
        bitmap.as_bitmap_ptr(),
        rotation,
        xscale,
        yscale,
        &mut _alloced_size,
      )
    };
    Bitmap::from_owned_ptr(bitmap_ptr)
  }

  pub fn from_file(path: &str) -> Result<Bitmap, Error> {
    let mut out_err: *const u8 = core::ptr::null_mut();

    // UNCLEAR: out_err is not a fixed string (it contains the name of the image). However, future
    // calls will overwrite the previous out_err and trying to free it via system->realloc crashes
    // (likely because the pointer wasn't alloc'd by us). This probably (hopefully??) means that we
    // don't need to free it.
    let bitmap_ptr = unsafe {
      CApiState::get().cgraphics.loadBitmap.unwrap()(
        path.to_null_terminated_utf8().as_ptr(),
        &mut out_err,
      )
    };

    if !out_err.is_null() {
      let result = unsafe { crate::null_terminated::parse_null_terminated_utf8(out_err) };
      match result {
        // A valid error string.
        Ok(err) => Err(format!("load_bitmap: {}", err).into()),
        // An invalid error string.
        Err(err) => Err(format!("load_bitmap: unknown error ({})", err).into()),
      }
    } else {
      assert!(!bitmap_ptr.is_null());
      Ok(Bitmap::from_owned_ptr(bitmap_ptr))
    }
  }

  /// Construct an Bitmap from an owning pointer.
  pub(crate) fn from_owned_ptr(bitmap_ptr: *mut CLCDBitmap) -> Self {
    Bitmap {
      owned: BitmapRef::from_ptr(bitmap_ptr),
    }
  }
}

impl Clone for Bitmap {
  fn clone(&self) -> Self {
    Bitmap::from_owned_ptr(unsafe {
      CApiState::get().cgraphics.copyBitmap.unwrap()(self.owned.bitmap_ptr)
    })
  }
}

impl Drop for Bitmap {
  fn drop(&mut self) {
    unsafe {
      CApiState::get().cgraphics.freeBitmap.unwrap()(self.owned.bitmap_ptr);
    }
  }
}

/// A reference to an `Bitmap`, which has a lifetime tied to a different `Bitmap` (or
/// `BitmapRef`) with a lifetime `'a`.
#[derive(Debug)]
pub struct SharedBitmapRef<'a> {
  bref: BitmapRef,
  _marker: core::marker::PhantomData<&'a Bitmap>,
}

impl SharedBitmapRef<'_> {
  /// Construct a SharedBitmapRef from a non-owning pointer.
  ///
  /// Requires being told the lifetime of the Bitmap this is making a reference to.
  pub(crate) fn from_ptr<'a>(bitmap_ptr: *mut CLCDBitmap) -> SharedBitmapRef<'a> {
    SharedBitmapRef {
      bref: BitmapRef::from_ptr(bitmap_ptr),
      _marker: core::marker::PhantomData,
    }
  }
}

impl Clone for SharedBitmapRef<'_> {
  fn clone(&self) -> Self {
    SharedBitmapRef::from_ptr(self.bref.bitmap_ptr)
  }
}

/// A borrow of an Bitmap (or SharedBitmap) is held as this type.
///
/// BitmapRef exposes most of the method of an Bitmap, allowing them to be used on an owned or
/// borrowed bitmap.
///
/// Intentionally not `Copy` as `BitmapRef` can only be referred to as a reference.
#[derive(Debug)]
pub struct BitmapRef {
  bitmap_ptr: *mut CLCDBitmap,
}

impl BitmapRef {
  /// Construct an BitmapRef from a non-owning pointer.
  pub(crate) fn from_ptr(bitmap_ptr: *mut CLCDBitmap) -> Self {
    BitmapRef { bitmap_ptr }
  }

  fn data_and_pixels_ptr(&self) -> (BitmapData, *mut u8) {
    let mut width = 0;
    let mut height = 0;
    let mut rowbytes = 0;
    let mut hasmask = 0;
    let mut pixels = core::ptr::null_mut();
    unsafe {
      CApiState::get().cgraphics.getBitmapData.unwrap()(
        self.bitmap_ptr,
        &mut width,
        &mut height,
        &mut rowbytes,
        &mut hasmask,
        &mut pixels,
      )
    };
    let data = BitmapData {
      width,
      height,
      rowbytes,
      hasmask,
    };
    (data, pixels)
  }

  /// Loads the image at `path` into the previously allocated `BitmapRef`.
  pub fn load_file(&mut self, path: &str) -> Result<(), Error> {
    let mut out_err: *const u8 = core::ptr::null_mut();

    // UNCLEAR: out_err is not a fixed string (it contains the name of the image). However, future
    // calls will overwrite the previous out_err and trying to free it via system->realloc crashes
    // (likely because the pointer wasn't alloc'd by us). This probably (hopefully??) means that we
    // don't need to free it.
    unsafe {
      CApiState::get().cgraphics.loadIntoBitmap.unwrap()(
        path.to_null_terminated_utf8().as_ptr(),
        self.as_bitmap_mut_ptr(),
        &mut out_err,
      )
    };

    if !out_err.is_null() {
      let result = unsafe { crate::null_terminated::parse_null_terminated_utf8(out_err) };
      match result {
        // A valid error string.
        Ok(err) => Err(format!("load_into_bitmap: {}", err).into()),
        // An invalid error string.
        Err(err) => Err(format!("load_into_bitmap: unknown error ({})", err).into()),
      }
    } else {
      Ok(())
    }
  }

  /// Returns the bitmap's metadata such as its width and height.
  pub fn data(&self) -> BitmapData {
    let (data, _) = self.data_and_pixels_ptr();
    data
  }

  /// Gives read acccess to the pixels of the bitmap as an array of bytes.
  ///
  /// Each byte represents 8 pixels, where each pixel is a bit. The highest bit is the leftmost
  /// pixel, and lowest bit is the rightmost.
  pub fn as_bytes(&self) -> &[u8] {
    let (data, pixels) = self.data_and_pixels_ptr();
    unsafe { core::slice::from_raw_parts(pixels, (data.rowbytes * data.height) as usize) }
  }
  /// Gives read-write acccess to the pixels of the bitmap as an array of bytes.
  ///
  /// Each byte represents 8 pixels, where each pixel is a bit. The highest bit is the leftmost
  /// pixel, and lowest bit is the rightmost.
  pub fn as_mut_bytes(&mut self) -> &mut [u8] {
    let (data, pixels) = self.data_and_pixels_ptr();
    unsafe { core::slice::from_raw_parts_mut(pixels, (data.rowbytes * data.height) as usize) }
  }
  /// Gives read acccess to the individual pixels of the bitmap.
  pub fn pixels(&self) -> BitmapPixels {
    let (data, pixels) = self.data_and_pixels_ptr();
    let slice =
      unsafe { core::slice::from_raw_parts(pixels, (data.rowbytes * data.height) as usize) };
    BitmapPixels {
      data,
      pixels: slice,
    }
  }
  /// Gives read-write acccess to the individual pixels of the bitmap.
  pub fn pixels_mut(&mut self) -> BitmapPixelsMut {
    let (data, pixels) = self.data_and_pixels_ptr();
    let slice =
      unsafe { core::slice::from_raw_parts_mut(pixels, (data.rowbytes * data.height) as usize) };
    BitmapPixelsMut {
      data,
      pixels: slice,
    }
  }

  /// Clears the bitmap, filling with the given `bgcolor`.
  pub fn clear<'a, C>(&mut self, bgcolor: C)
  where
    Color<'a>: From<C>,
  {
    unsafe {
      CApiState::get().cgraphics.clearBitmap.unwrap()(
        self.bitmap_ptr,
        Color::<'a>::from(bgcolor).to_c_color(),
      );
    }
  }

  /// Sets a mask image for the given bitmap. The set mask must be the same size as the target
  /// bitmap.
  ///
  /// The mask bitmap is copied, so no reference is held to it.
  pub fn set_mask_bitmap(&mut self, mask: &BitmapRef) -> Result<(), Error> {
    // Playdate makes a copy of the mask bitmap.
    let result = unsafe {
      CApiState::get().cgraphics.setBitmapMask.unwrap()(self.bitmap_ptr, mask.bitmap_ptr)
    };
    match result {
      1 => Ok(()),
      0 => Err("failed to set mask bitmap, dimensions to not match".into()),
      _ => panic!("unknown error result from setBitmapMask"),
    }
  }

  /// The mask bitmap attached to this bitmap.
  ///
  /// Returns the mask bitmap, if one has been attached with `set_mask_bitmap()`, or None.
  pub fn mask_bitmap(&self) -> Option<SharedBitmapRef> {
    let mask = unsafe {
      // Playdate owns the mask bitmap, and reference a pointer to it. Playdate would free the mask
      // presumably when `self` is freed.
      CApiState::get().cgraphics.getBitmapMask.unwrap()(self.bitmap_ptr)
    };
    if !mask.is_null() {
      Some(SharedBitmapRef::from_ptr(mask))
    } else {
      None
    }
  }

  pub(crate) unsafe fn as_bitmap_ptr(&self) -> *mut CLCDBitmap {
    self.bitmap_ptr
  }
  pub(crate) unsafe fn as_bitmap_mut_ptr(&mut self) -> *mut CLCDBitmap {
    self.bitmap_ptr
  }
}

impl core::ops::Deref for Bitmap {
  type Target = BitmapRef;

  fn deref(&self) -> &Self::Target {
    &self.owned
  }
}
impl core::ops::DerefMut for Bitmap {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.owned
  }
}

impl core::borrow::Borrow<BitmapRef> for Bitmap {
  fn borrow(&self) -> &BitmapRef {
    self // This calls Deref.
  }
}
impl core::borrow::BorrowMut<BitmapRef> for Bitmap {
  fn borrow_mut(&mut self) -> &mut BitmapRef {
    self // This calls DerefMut.
  }
}

impl alloc::borrow::ToOwned for BitmapRef {
  type Owned = Bitmap;

  fn to_owned(&self) -> Self::Owned {
    Bitmap::from_owned_ptr(unsafe {
      CApiState::get().cgraphics.copyBitmap.unwrap()(self.bitmap_ptr)
    })
  }
}

impl core::ops::Deref for SharedBitmapRef<'_> {
  type Target = BitmapRef;

  fn deref(&self) -> &Self::Target {
    &self.bref
  }
}
impl core::ops::DerefMut for SharedBitmapRef<'_> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.bref
  }
}

impl core::borrow::Borrow<BitmapRef> for SharedBitmapRef<'_> {
  fn borrow(&self) -> &BitmapRef {
    self // This calls Deref.
  }
}
impl core::borrow::BorrowMut<BitmapRef> for SharedBitmapRef<'_> {
  fn borrow_mut(&mut self) -> &mut BitmapRef {
    self // This calls DerefMut.
  }
}

impl AsRef<BitmapRef> for Bitmap {
  fn as_ref(&self) -> &BitmapRef {
    self // This calls Deref.
  }
}
impl AsMut<BitmapRef> for Bitmap {
  fn as_mut(&mut self) -> &mut BitmapRef {
    self // This calls DerefMut.
  }
}
impl AsRef<BitmapRef> for SharedBitmapRef<'_> {
  fn as_ref(&self) -> &BitmapRef {
    self // This calls Deref.
  }
}
impl AsMut<BitmapRef> for SharedBitmapRef<'_> {
  fn as_mut(&mut self) -> &mut BitmapRef {
    self // This calls DerefMut.
  }
}
impl AsRef<BitmapRef> for BitmapRef {
  fn as_ref(&self) -> &BitmapRef {
    self
  }
}
impl AsMut<BitmapRef> for BitmapRef {
  fn as_mut(&mut self) -> &mut BitmapRef {
    self
  }
}

/// Metadata for an `Bitmap`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BitmapData {
  width: i32,
  height: i32,
  rowbytes: i32,
  hasmask: i32,
}
impl BitmapData {
  /// The number of pixels (or, columns) per row of the bitmap.
  ///
  /// Each pixel is a single bit, and there may be more bytes (as determined by `row_bytes()`) in a
  /// row than required to hold all the pixels.
  pub fn width(&self) -> i32 {
    self.width
  }
  /// The number of rows in the bitmap.
  pub fn height(&self) -> i32 {
    self.height
  }
  /// The number of bytes per row of the bitmap.
  pub fn row_bytes(&self) -> i32 {
    self.rowbytes
  }
  /// Whether the bitmap has a mask attached, via `set_mask_bitmap()`.
  pub fn has_mask(&self) -> bool {
    self.hasmask != 0
  }
}

/// Provide readonly access to the pixels in an Bitmap, through its BitmapData.
pub struct BitmapPixels<'bitmap> {
  data: BitmapData,
  pixels: &'bitmap [u8],
}
impl BitmapPixels<'_> {
  pub fn get(&self, x: usize, y: usize) -> bool {
    let byte_index = self.data.rowbytes as usize * y + x / 8;
    let bit_index = x % 8;
    (self.pixels[byte_index] >> (7 - bit_index)) & 0x1 != 0
  }
}

/// Provide mutable access to the pixels in an Bitmap, through its BitmapData.
pub struct BitmapPixelsMut<'bitmap> {
  data: BitmapData,
  pixels: &'bitmap mut [u8],
}
impl BitmapPixelsMut<'_> {
  pub fn get(&self, x: usize, y: usize) -> bool {
    BitmapPixels {
      data: self.data,
      pixels: self.pixels,
    }
    .get(x, y)
  }
  pub fn set(&mut self, x: usize, y: usize, new_value: bool) {
    let byte_index = self.data.rowbytes as usize * y + x / 8;
    let bit_index = x % 8;
    if new_value {
      self.pixels[byte_index] |= 1u8 << (7 - bit_index);
    } else {
      self.pixels[byte_index] &= !(1u8 << (7 - bit_index));
    }
  }
}
