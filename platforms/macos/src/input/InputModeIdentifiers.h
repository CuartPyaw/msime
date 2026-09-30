#pragma once
#import <Foundation/Foundation.h>

// The four input modes Info.plist.in declares. Each shows the bundle's logo with its own 中, 日, 한 or 英 badge; the selected one is checked in the input menu and named in the input-source list. info-plist-names checks these literals against the plist.
static NSString *const MSIMEChineseInputModeID = @"app.msime.inputmethod.MetasequoiaIME.Hans";
static NSString *const MSIMEEnglishInputModeID = @"app.msime.inputmethod.MetasequoiaIME.Roman";
static NSString *const MSIMEJapaneseInputModeID = @"app.msime.inputmethod.MetasequoiaIME.Japanese";
static NSString *const MSIMEKoreanInputModeID = @"app.msime.inputmethod.MetasequoiaIME.Korean";

// The mode the menu bar shows. English is the controller passing keys through; the other three follow the scheme behind it.
enum class MSIMEInputMode { Chinese, English, Japanese, Korean };

// English mode wins over the scheme: 英 is shown whenever the controller passes keys through, whatever scheme is behind it. Otherwise the japanese scheme shows 日, the korean scheme shows 한 and every Chinese scheme shows 中.
static inline MSIMEInputMode MSIMEInputModeFor(BOOL english, NSString *scheme) {
    if (english) return MSIMEInputMode::English;
    if ([scheme isEqualToString:@"japanese"]) return MSIMEInputMode::Japanese;
    if ([scheme isEqualToString:@"korean"]) return MSIMEInputMode::Korean;
    return MSIMEInputMode::Chinese;
}

static inline NSString *MSIMEInputModeID(MSIMEInputMode mode) {
    switch (mode) {
    case MSIMEInputMode::English: return MSIMEEnglishInputModeID;
    case MSIMEInputMode::Japanese: return MSIMEJapaneseInputModeID;
    case MSIMEInputMode::Korean: return MSIMEKoreanInputModeID;
    case MSIMEInputMode::Chinese: break;
    }
    return MSIMEChineseInputModeID;
}

static inline BOOL MSIMEIsInputModeID(id value) {
    return [value isKindOfClass:NSString.class] &&
           ([value isEqualToString:MSIMEChineseInputModeID] || [value isEqualToString:MSIMEEnglishInputModeID] ||
            [value isEqualToString:MSIMEJapaneseInputModeID] || [value isEqualToString:MSIMEKoreanInputModeID]);
}

// The mode an identifier names. Anything that is not one of this bundle's modes reads as Chinese; callers check MSIMEIsInputModeID first when that matters.
static inline MSIMEInputMode MSIMEInputModeForID(NSString *identifier) {
    if ([identifier isEqualToString:MSIMEEnglishInputModeID]) return MSIMEInputMode::English;
    if ([identifier isEqualToString:MSIMEJapaneseInputModeID]) return MSIMEInputMode::Japanese;
    if ([identifier isEqualToString:MSIMEKoreanInputModeID]) return MSIMEInputMode::Korean;
    return MSIMEInputMode::Chinese;
}

// The scheme a mode selects, or nil for English, which leaves the scheme alone. 中 has no scheme of its own: it goes back to the Chinese scheme the user left, which only the preferences know.
static inline NSString *MSIMESchemeForInputMode(MSIMEInputMode mode) {
    switch (mode) {
    case MSIMEInputMode::Japanese: return @"japanese";
    case MSIMEInputMode::Korean: return @"korean";
    case MSIMEInputMode::Chinese:
    case MSIMEInputMode::English: break;
    }
    return nil;
}

// Keeps the system's selected input mode and the controller's Chinese/English/Japanese/Korean state in step without either side echoing the other. The selected mode is global to the login session, so one state serves every controller instance.
//
// `current` is the mode last reported by the system or last requested by the controller; a report that repeats it is the system confirming what is already shown, not a user choice, so it does not flip the controller's state. `selecting` is set while the controller is asking the client to switch, so a report delivered synchronously from inside that call is not treated as a new choice either.
struct MSIMESystemInputModeState {
    NSString *current = nil;
    bool selecting = false;
};

// The system's selected input mode is global to the login session, so every controller instance shares one record of it. Leaving the input method clears it, so the first report after coming back is adopted even if it names the mode shown before leaving.
inline MSIMESystemInputModeState &MSIMESharedSystemInputModeState() {
    static MSIMESystemInputModeState state;
    return state;
}

static inline void MSIMEResetSystemInputModeState(MSIMESystemInputModeState &state) { state = MSIMESystemInputModeState{}; }

// Records a mode the system reported through setValue:forTag:client:. Returns YES when the controller should adopt it: a known mode that differs from the one already shown and that the controller did not just request itself.
static inline BOOL MSIMEAdoptReportedInputMode(MSIMESystemInputModeState &state, id value) {
    if (!MSIMEIsInputModeID(value)) return NO;
    const BOOL changed = ![value isEqualToString:state.current];
    state.current = [value copy];
    return changed && !state.selecting;
}

// Whether the system offers a mode for selection. A mode the user removed in System Settings, or one an install from before the English, Japanese or Korean mode existed has not registered yet, cannot be selected, and asking for it would leave `current` naming a mode the menu bar does not show - the next report of the real one would then flip the controller's state back.
using MSIMEInputModeAvailability = BOOL (*)(NSString *identifier);

// Asks the client to show `mode`, unless it already does or the system does not offer it. Returns whether a switch was requested. The japanese and korean schemes fall back to 中 when their mode is not offered: the controller is still composing through the Engine, and 中 is closer to that than an 英 left over from before. A Japanese or Korean mode the system has just reported as shown is offered by definition, so it is kept rather than second-guessed.
static inline BOOL MSIMESelectSystemInputMode(MSIMESystemInputModeState &state, NSString *mode, id client,
                                              MSIMEInputModeAvailability available) {
    if (!available) return NO;
    if (([mode isEqualToString:MSIMEJapaneseInputModeID] || [mode isEqualToString:MSIMEKoreanInputModeID]) &&
        ![mode isEqualToString:state.current] && !available(mode))
        mode = MSIMEChineseInputModeID;
    if (state.selecting || [mode isEqualToString:state.current] || ![client respondsToSelector:@selector(selectInputMode:)] ||
        !available(mode))
        return NO;
    state.current = mode;
    state.selecting = true;
    [client performSelector:@selector(selectInputMode:) withObject:mode];
    state.selecting = false;
    return YES;
}
