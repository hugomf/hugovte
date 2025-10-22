#import <gtk/gtk.h>
#import <gdk/macos/gdkmacos.h>
#import <CoreGraphics/CoreGraphics.h>
#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdint.h>

// Private CoreGraphics API types
typedef uint32_t CGSConnectionID;
typedef uint32_t CGSWindowID;

// Function pointer types for private APIs
typedef CGSConnectionID (*CGSDefaultConnectionForThreadFunc)(void);
typedef int32_t (*CGSSetWindowBackgroundBlurRadiusFunc)(CGSConnectionID, CGSWindowID, uint32_t);

// Global function pointers
static CGSDefaultConnectionForThreadFunc pCGSDefaultConnectionForThread = NULL;
static CGSSetWindowBackgroundBlurRadiusFunc pCGSSetWindowBackgroundBlurRadius = NULL;
static CGSConnectionID connection_id = 0;

// Initialize blur API (must be called once before using blur)
void init_blur_api(void) {
    if (pCGSSetWindowBackgroundBlurRadius != NULL) {
        return; // Already initialized
    }
    
    void* handle = dlopen("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", RTLD_LAZY);
    if (!handle) {
        fprintf(stderr, "Failed to load CoreGraphics framework\n");
        return;
    }
    
    pCGSDefaultConnectionForThread = (CGSDefaultConnectionForThreadFunc)dlsym(handle, "CGSDefaultConnectionForThread");
    pCGSSetWindowBackgroundBlurRadius = (CGSSetWindowBackgroundBlurRadiusFunc)dlsym(handle, "CGSSetWindowBackgroundBlurRadius");
    
    if (pCGSDefaultConnectionForThread && pCGSSetWindowBackgroundBlurRadius) {
        connection_id = pCGSDefaultConnectionForThread();
        printf("✅ Blur API initialized (connection: %u)\n", connection_id);
    } else {
        fprintf(stderr, "❌ Failed to load CGS functions\n");
    }
}


// Set window opacity and blur in one call
// opacity: 0.0 to 1.0 (0.0 = fully transparent, 1.0 = fully opaque)
// blur_amount: 0.0 to 1.0 (0.0 = no blur, 1.0 = maximum blur)
int set_opacity_and_blur(void* gtk_window_ptr, double opacity, double blur_amount, double red, double green, double blue) {
    @autoreleasepool {
        GtkWindow* gtk_window = (GtkWindow*)gtk_window_ptr;
        GdkSurface* surface = gtk_native_get_surface(GTK_NATIVE(gtk_window));
        
        if (!GDK_IS_MACOS_SURFACE(surface)) {
            return -1;
        }
        
        NSWindow* ns_window = (__bridge NSWindow*)gdk_macos_surface_get_native_window(GDK_MACOS_SURFACE(surface));
        
        // Set window transparency
        [ns_window setOpaque:NO];
        [ns_window setTitlebarAppearsTransparent:NO];
        [ns_window setBackgroundColor:[NSColor clearColor]];
        
        // Always show shadow/border regardless of opacity
        [ns_window setHasShadow:YES];
        
        // Set content view opacity (using a semi-transparent black/white)
        NSView* contentView = [ns_window contentView];
        [contentView setWantsLayer:YES];
        [[contentView layer] setOpaque:NO];
        
        // Use opacity to create the background (you can change the color here)
       //NSColor* backgroundColor = [NSColor colorWithWhite:0.0 alpha:1.0 - opacity];
        NSColor* backgroundColor = [NSColor colorWithRed:red green:green blue:blue alpha:1.0 - opacity];


        [[contentView layer] setBackgroundColor:[backgroundColor CGColor]];
        
        // Apply blur if requested and API is available
        // Convert blur_amount (0.0-1.0) to radius (0-100)
        uint32_t blur_radius = (uint32_t)(blur_amount * 100.0);
        if (blur_radius > 0 && connection_id != 0 && pCGSSetWindowBackgroundBlurRadius) {
            NSInteger window_number = [ns_window windowNumber];
            pCGSSetWindowBackgroundBlurRadius(connection_id, (CGSWindowID)window_number, blur_radius);
        }
        
        [ns_window display];
        [ns_window invalidateShadow];
        
        return 0;
    }
}

