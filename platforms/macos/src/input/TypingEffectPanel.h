#pragma once

#import <AppKit/AppKit.h>
#include <stdint.h>

NS_ASSUME_NONNULL_BEGIN

/// The effect style msime_client_typing_effect reports in bits 17-19, the codes of client-core's EffectStyle.
typedef NS_ENUM(NSUInteger, MSIMETypingEffectStyle) {
    MSIMETypingEffectStyleOff = 0,
    MSIMETypingEffectStyleFlash = 1,
    MSIMETypingEffectStyleSparks = 2,
    MSIMETypingEffectStylePowerMode = 3,
};

/// msime_client_typing_effect event codes: the key class (0 any other key, 1 space, 2 enter, 3 backspace, as msime_client_key_sound takes them) or a commit.
static const uint32_t MSIMETypingEffectEventCommit = 4;
/// Event flag: count the key but queue no tier-up sound, because a full-screen application has the display.
static const uint32_t MSIMETypingEffectEventMuted = 0x200;
/// Answer bit: this key reached a new combo tier.
static const uint32_t MSIMETypingEffectTierUp = 0x10000;

/// One msime_client_typing_effect answer, unpacked.
typedef struct {
    NSUInteger combo;
    BOOL tierUp;
    MSIMETypingEffectStyle style;
} MSIMETypingEffect;

FOUNDATION_EXPORT MSIMETypingEffect MSIMETypingEffectDecode(uint32_t packed);
/// The style actually drawn. Sparks and Power Mode move things across the screen, so they fall back to Flash while the user asks for reduced motion or the Mac is in Low Power Mode.
FOUNDATION_EXPORT MSIMETypingEffectStyle MSIMETypingEffectDrawnStyle(MSIMETypingEffectStyle style, BOOL reduceMotion, BOOL lowPower);
/// The combo badge text, or nil below two keys: a count of one is not a combo yet.
FOUNDATION_EXPORT NSString *_Nullable MSIMETypingEffectComboText(NSUInteger combo);

/// The candidate card's flash: a tint over the card that fades out. Takes no clicks and is not an accessibility element.
@interface MSIMETypingEffectFlashView : NSView
@end

/// Typing effects drawn beside the candidate card: sparks at the caret, a flash over the card and the combo badge on its corner. A non-activating, click-through panel under the candidate window. It holds no timer and no running emitter once an effect has played out: the emitter's birth rate goes back to 0 and the panel leaves the screen.
@interface MSIMETypingEffectPanel : NSPanel
+ (instancetype)sharedPanel;
/// Reads `plugins.effect_style`, `plugins.combo_counter` and `plugins.effect_intensity`. A document without `plugins` leaves the current settings alone; turning everything off takes the panel off screen.
- (void)applyPreferences:(NSDictionary *)preferences;
/// Whether there is anything to draw: a style other than off, or the combo counter. The controller asks the library for nothing while this is NO.
@property(nonatomic, readonly) BOOL configured;
/// `plugins.effect_intensity`, 0-100; 50 until set.
@property(nonatomic, readonly) NSUInteger intensity;
/// Draws one answer of msime_client_typing_effect. `caretRect` is the client's line rectangle at the caret in screen coordinates. `candidateView` is the candidate window's content while it is on screen, with the card's rectangle in that view and the card's corner radius; nil when no card is shown.
- (void)presentEffect:(uint32_t)packed
               commit:(BOOL)commit
            caretRect:(NSRect)caretRect
        candidateView:(nullable NSView *)candidateView
             cardRect:(NSRect)cardRect
         cornerRadius:(CGFloat)cornerRadius;
/// The same, with the system's Reduce Motion and Low Power Mode answers given instead of read.
- (void)presentEffect:(uint32_t)packed
               commit:(BOOL)commit
            caretRect:(NSRect)caretRect
        candidateView:(nullable NSView *)candidateView
             cardRect:(NSRect)cardRect
         cornerRadius:(CGFloat)cornerRadius
         reduceMotion:(BOOL)reduceMotion
             lowPower:(BOOL)lowPower;
/// The idle state: no particle is born, the emitter and badge are hidden and the panel is off screen. Called when an effect has played out, on focus loss and when the settings turn effects off.
- (void)settle;
/// YES while the emitter is giving birth to sparks.
@property(nonatomic, readonly) BOOL emitting;
/// The style the last effect was drawn in, after the Reduce Motion and Low Power fallback.
@property(nonatomic, readonly) MSIMETypingEffectStyle drawnStyle;
/// The combo badge text while the badge is on screen, otherwise nil.
@property(nonatomic, copy, readonly, nullable) NSString *displayedCombo;
@end

NS_ASSUME_NONNULL_END
