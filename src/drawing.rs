use std::collections::HashMap;

// src/drawing.rs
use cairo::{Context, FontSlant, FontWeight, ScaledFont, ImageSurface, Format};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FontKey {
    slant: FontSlant,
    weight: FontWeight,
}

// Manual Hash implementation since FontSlant and FontWeight don't implement Hash
impl std::hash::Hash for FontKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Convert enums to their discriminant values for hashing
        std::mem::discriminant(&self.slant).hash(state);
        std::mem::discriminant(&self.weight).hash(state);
    }
}

pub struct DrawingCache {
    font_family: String,
    font_size: f64,
    fonts: HashMap<FontKey, ScaledFont>,
    char_width: f64,
    char_height: f64,
    ascent: f64,
}

impl DrawingCache {
    pub fn new(font_family: &str, font_size: f64) -> Result<Self, cairo::Error> {
        let surf = ImageSurface::create(Format::ARgb32, 1, 1)?;
        let cr = Context::new(&surf)?;
        
        // Pre-create scaled fonts for common combinations
        let mut fonts = HashMap::new();
        
        let combinations = [
            (FontSlant::Normal, FontWeight::Normal),
            (FontSlant::Normal, FontWeight::Bold),
            (FontSlant::Italic, FontWeight::Normal),
            (FontSlant::Italic, FontWeight::Bold),
        ];
        
        for (slant, weight) in combinations {
            let key = FontKey { slant, weight };
            let font = Self::create_scaled_font(&cr, font_family, font_size, slant, weight)?;
            fonts.insert(key, font);
        }
        
        // Calculate character metrics using normal font
        let normal_font = fonts.get(&FontKey { slant: FontSlant::Normal, weight: FontWeight::Normal })
            .unwrap();
        let extents = normal_font.text_extents("M");
        
        Ok(Self {
            font_family: font_family.to_string(),
            font_size,
            fonts,
            char_width: extents.width(),
            char_height: extents.height(),
            ascent: extents.y_bearing().abs(),
        })
    }
    
    fn create_scaled_font(
        cr: &Context,
        family: &str,
        size: f64,
        slant: FontSlant,
        weight: FontWeight,
    ) -> Result<ScaledFont, cairo::Error> {
        cr.select_font_face(family, slant, weight);
        cr.set_font_size(size);
        
        // Create scaled font using the context's current font settings
        let font_face = cr.font_face().clone();
        let font_matrix = cr.font_matrix();
        let ctm = cr.matrix();
        let options = cairo::FontOptions::new().map_err(|_| cairo::Error::FontTypeMismatch)?;
        
        ScaledFont::new(&font_face, &font_matrix, &ctm, &options)
    }
    
    pub fn get_font(&self, slant: FontSlant, weight: FontWeight) -> Option<&ScaledFont> {
        self.fonts.get(&FontKey { slant, weight })
    }
    
    pub fn char_width(&self) -> f64 {
        self.char_width
    }
    
    pub fn char_height(&self) -> f64 {
        self.char_height
    }
    
    pub fn ascent(&self) -> f64 {
        self.ascent
    }
    
    pub fn font_size(&self) -> f64 {
        self.font_size
    }
    
    pub fn font_family(&self) -> &str {
        &self.font_family
    }
}

impl Clone for DrawingCache {
    fn clone(&self) -> Self {
        // Recreate the drawing cache - this is a bit expensive but necessary
        DrawingCache::new(&self.font_family, self.font_size)
            .expect("Failed to clone DrawingCache")
    }
}