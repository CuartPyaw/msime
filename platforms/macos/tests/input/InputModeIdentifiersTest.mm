#import "../../src/input/InputModeIdentifiers.h"

#include <cstdio>
#include <cstdlib>

// Stands in for the client: records every selectInputMode: and, like the system, can report the new mode back from inside the call.
@interface InputModeRecordingClient : NSObject
@property(nonatomic, strong) NSMutableArray<NSString *> *selected;
@property(nonatomic) MSIMESystemInputModeState *state;
@property(nonatomic) BOOL echoes;
@property(nonatomic) BOOL echoAdopted;
- (void)selectInputMode:(NSString *)identifier;
@end

@implementation InputModeRecordingClient
- (instancetype)init {
    if ((self = [super init])) _selected = [NSMutableArray array];
    return self;
}
- (void)selectInputMode:(NSString *)identifier {
    [self.selected addObject:identifier];
    if (self.echoes && self.state && MSIMEAdoptReportedInputMode(*self.state, identifier)) self.echoAdopted = YES;
}
@end

namespace {
BOOL gEnglishModeEnabled = YES;
BOOL gJapaneseModeEnabled = YES;
BOOL gKoreanModeEnabled = YES;
BOOL Available(NSString *identifier) {
    if ([identifier isEqualToString:MSIMEJapaneseInputModeID]) return gJapaneseModeEnabled;
    if ([identifier isEqualToString:MSIMEKoreanInputModeID]) return gKoreanModeEnabled;
    return [identifier isEqualToString:MSIMEChineseInputModeID] || gEnglishModeEnabled;
}

void require(bool condition, const char *message) {
    if (!condition) {
        std::fprintf(stderr, "%s\n", message);
        std::exit(1);
    }
}
} // namespace

int main() {
    @autoreleasepool {
        require([MSIMEInputModeID(MSIMEInputModeFor(NO, @"quanpin")) isEqualToString:@"app.msime.inputmethod.MetasequoiaIME.Hans"] &&
                    [MSIMEInputModeID(MSIMEInputModeFor(YES, @"quanpin")) isEqualToString:@"app.msime.inputmethod.MetasequoiaIME.Roman"] &&
                    [MSIMEInputModeID(MSIMEInputModeFor(NO, @"japanese")) isEqualToString:@"app.msime.inputmethod.MetasequoiaIME.Japanese"] &&
                    [MSIMEInputModeID(MSIMEInputModeFor(NO, @"korean")) isEqualToString:@"app.msime.inputmethod.MetasequoiaIME.Korean"],
                "The Chinese, English, Japanese and Korean states do not map to the modes Info.plist.in declares.");
        require(MSIMEInputModeFor(NO, @"shuangpin") == MSIMEInputMode::Chinese && MSIMEInputModeFor(NO, @"wubi") == MSIMEInputMode::Chinese &&
                    MSIMEInputModeFor(NO, nil) == MSIMEInputMode::Chinese,
                "A Chinese scheme did not show 中.");
        require(MSIMEInputModeFor(YES, @"japanese") == MSIMEInputMode::English && MSIMEInputModeFor(YES, @"korean") == MSIMEInputMode::English,
                "English mode over the japanese or korean scheme did not show 英.");
        for (NSString *identifier in @[MSIMEChineseInputModeID, MSIMEEnglishInputModeID, MSIMEJapaneseInputModeID, MSIMEKoreanInputModeID])
            require([MSIMEInputModeID(MSIMEInputModeForID(identifier)) isEqualToString:identifier],
                    "A mode identifier does not map back to the mode it names.");
        require(MSIMEInputModeForID(@"com.apple.keylayout.ABC") == MSIMEInputMode::Chinese,
                "An unknown identifier did not read as the Chinese mode.");
        require([MSIMESchemeForInputMode(MSIMEInputMode::Japanese) isEqualToString:@"japanese"] &&
                    [MSIMESchemeForInputMode(MSIMEInputMode::Korean) isEqualToString:@"korean"] &&
                    MSIMESchemeForInputMode(MSIMEInputMode::Chinese) == nil && MSIMESchemeForInputMode(MSIMEInputMode::English) == nil,
                "A mode selects the wrong scheme.");
        require(MSIMEIsInputModeID(MSIMEChineseInputModeID) && MSIMEIsInputModeID(MSIMEEnglishInputModeID) &&
                    MSIMEIsInputModeID(MSIMEJapaneseInputModeID) && MSIMEIsInputModeID(MSIMEKoreanInputModeID) &&
                    !MSIMEIsInputModeID(@"com.apple.keylayout.ABC") && !MSIMEIsInputModeID(@"app.msime.inputmethod.MetasequoiaIME") &&
                    !MSIMEIsInputModeID(@42) && !MSIMEIsInputModeID(nil),
                "Something other than this bundle's four modes was taken for one of them.");

        MSIMESystemInputModeState state;
        InputModeRecordingClient *client = [InputModeRecordingClient new];
        client.state = &state;
        client.echoes = YES;

        // Activation with nothing recorded selects the stored state, and the system's synchronous echo is not a new choice.
        require(MSIMESelectSystemInputMode(state, MSIMEChineseInputModeID, client, Available) && [client.selected isEqualToArray:@[MSIMEChineseInputModeID]],
                "The first alignment did not select the Chinese mode.");
        require(!client.echoAdopted && !state.selecting, "The echo of the controller's own selection was adopted.");
        require(!MSIMESelectSystemInputMode(state, MSIMEChineseInputModeID, client, Available) && client.selected.count == 1,
                "Aligning to the mode already shown asked the client again.");

        // A Shift toggle into English selects the English mode once.
        require(MSIMESelectSystemInputMode(state, MSIMEEnglishInputModeID, client, Available) &&
                    [client.selected.lastObject isEqualToString:MSIMEEnglishInputModeID] && client.selected.count == 2,
                "Toggling into English did not select the English mode.");
        require(!client.echoAdopted, "The echo of the toggle flipped the state back.");
        require(!MSIMEAdoptReportedInputMode(state, MSIMEEnglishInputModeID),
                "A later report that only repeats the shown mode was adopted as a change.");

        // The user picks the Chinese entry from the input menu: adopt it, and the resulting state sync does not call back.
        require(MSIMEAdoptReportedInputMode(state, MSIMEChineseInputModeID), "A mode picked from the input menu was not adopted.");
        require(!MSIMESelectSystemInputMode(state, MSIMEChineseInputModeID, client, Available) && client.selected.count == 2,
                "Adopting a system-reported mode echoed it back through selectInputMode:.");

        // setValue:forTag: with the English mode turns English on without selecting it back.
        require(MSIMEAdoptReportedInputMode(state, MSIMEEnglishInputModeID) &&
                    MSIMEInputModeForID(MSIMEEnglishInputModeID) == MSIMEInputMode::English,
                "Reporting the English mode did not turn English on.");
        require(!MSIMESelectSystemInputMode(state, MSIMEEnglishInputModeID, client, Available) && client.selected.count == 2,
                "Turning English on from a report called selectInputMode: back.");

        // An unknown identifier is ignored and leaves the record alone.
        require(!MSIMEAdoptReportedInputMode(state, @"com.apple.keylayout.ABC") && !MSIMEAdoptReportedInputMode(state, nil) &&
                    [state.current isEqualToString:MSIMEEnglishInputModeID],
                "An unknown mode identifier was adopted or overwrote the record.");

        // A report delivered while the controller is itself selecting is recorded but not adopted.
        state.selecting = true;
        require(!MSIMEAdoptReportedInputMode(state, MSIMEChineseInputModeID) && [state.current isEqualToString:MSIMEChineseInputModeID],
                "A report from inside the controller's own switch was adopted.");
        state.selecting = false;

        // When the English mode is not enabled - removed in System Settings, or not yet registered - nothing is requested and the record keeps naming the mode actually shown.
        gEnglishModeEnabled = NO;
        require(!MSIMESelectSystemInputMode(state, MSIMEEnglishInputModeID, client, Available) && client.selected.count == 2 &&
                    [state.current isEqualToString:MSIMEChineseInputModeID],
                "An unavailable mode was requested or recorded as shown.");
        require(!MSIMEAdoptReportedInputMode(state, MSIMEChineseInputModeID),
                "With the English mode unavailable, a report of the Chinese mode flipped English off.");
        gEnglishModeEnabled = YES;

        // The japanese scheme selects 日 once, and a later pick of 日 from the menu after 中 is adopted.
        require(MSIMESelectSystemInputMode(state, MSIMEJapaneseInputModeID, client, Available) &&
                    [client.selected.lastObject isEqualToString:MSIMEJapaneseInputModeID] && client.selected.count == 3 &&
                    !client.echoAdopted,
                "Switching to the japanese scheme did not select the Japanese mode, or its echo was adopted.");
        require(MSIMEAdoptReportedInputMode(state, MSIMEChineseInputModeID) && MSIMEAdoptReportedInputMode(state, MSIMEJapaneseInputModeID),
                "Picking 中 and then 日 from the input menu was not adopted.");
        require(!MSIMESelectSystemInputMode(state, MSIMEJapaneseInputModeID, client, Available) && client.selected.count == 3,
                "Adopting a reported Japanese mode echoed it back through selectInputMode:.");
        MSIMEAdoptReportedInputMode(state, MSIMEEnglishInputModeID);

        // An install that has not registered the Japanese mode shows 中 for the japanese scheme rather than leaving 英 up.
        gJapaneseModeEnabled = NO;
        require(MSIMESelectSystemInputMode(state, MSIMEJapaneseInputModeID, client, Available) &&
                    [client.selected.lastObject isEqualToString:MSIMEChineseInputModeID] && client.selected.count == 4 &&
                    [state.current isEqualToString:MSIMEChineseInputModeID],
                "An unavailable Japanese mode did not fall back to the Chinese mode.");
        require(!MSIMESelectSystemInputMode(state, MSIMEJapaneseInputModeID, client, Available) && client.selected.count == 4,
                "The Chinese fallback for an unavailable Japanese mode asked the client again.");
        require(MSIMEAdoptReportedInputMode(state, MSIMEJapaneseInputModeID) &&
                    !MSIMESelectSystemInputMode(state, MSIMEJapaneseInputModeID, client, Available) && client.selected.count == 4,
                "A Japanese mode the system reported as shown was replaced by the Chinese fallback.");
        gJapaneseModeEnabled = YES;
        MSIMEAdoptReportedInputMode(state, MSIMEChineseInputModeID);

        // The korean scheme selects 한 once, and an install that has not registered the Korean mode shows 中 for it, as for Japanese.
        const NSUInteger beforeKorean = client.selected.count;
        require(MSIMESelectSystemInputMode(state, MSIMEKoreanInputModeID, client, Available) &&
                    [client.selected.lastObject isEqualToString:MSIMEKoreanInputModeID] && client.selected.count == beforeKorean + 1 &&
                    !client.echoAdopted,
                "Switching to the korean scheme did not select the Korean mode, or its echo was adopted.");
        require(!MSIMESelectSystemInputMode(state, MSIMEKoreanInputModeID, client, Available) && client.selected.count == beforeKorean + 1,
                "Aligning to the Korean mode already shown asked the client again.");
        MSIMEAdoptReportedInputMode(state, MSIMEEnglishInputModeID);
        gKoreanModeEnabled = NO;
        require(MSIMESelectSystemInputMode(state, MSIMEKoreanInputModeID, client, Available) &&
                    [client.selected.lastObject isEqualToString:MSIMEChineseInputModeID] && client.selected.count == beforeKorean + 2 &&
                    [state.current isEqualToString:MSIMEChineseInputModeID],
                "An unavailable Korean mode did not fall back to the Chinese mode.");
        require(MSIMEAdoptReportedInputMode(state, MSIMEKoreanInputModeID) &&
                    !MSIMESelectSystemInputMode(state, MSIMEKoreanInputModeID, client, Available) && client.selected.count == beforeKorean + 2,
                "A Korean mode the system reported as shown was replaced by the Chinese fallback.");
        gKoreanModeEnabled = YES;
        MSIMEAdoptReportedInputMode(state, MSIMEChineseInputModeID);

        // Leaving the input method clears the record, so picking the entry shown before leaving is adopted on the way back instead of being taken for an echo.
        require(MSIMEAdoptReportedInputMode(state, MSIMEEnglishInputModeID) && !MSIMEAdoptReportedInputMode(state, MSIMEEnglishInputModeID),
                "The English report before leaving was not recorded.");
        MSIMEResetSystemInputModeState(state);
        require(state.current == nil && !state.selecting, "Resetting left the record naming a mode.");
        require(MSIMEAdoptReportedInputMode(state, MSIMEEnglishInputModeID) && [state.current isEqualToString:MSIMEEnglishInputModeID],
                "A report after leaving the input method was not adopted.");
        MSIMEResetSystemInputModeState(MSIMESharedSystemInputModeState());
        require(MSIMESharedSystemInputModeState().current == nil, "The shared record did not reset.");
        MSIMEAdoptReportedInputMode(state, MSIMEChineseInputModeID);

        // A client that cannot switch modes is left alone.
        require(!MSIMESelectSystemInputMode(state, MSIMEEnglishInputModeID, [NSObject new], Available) && [state.current isEqualToString:MSIMEChineseInputModeID],
                "A client without selectInputMode: was recorded as switched.");
        require(!MSIMESelectSystemInputMode(state, MSIMEEnglishInputModeID, nil, Available), "A missing client was asked to switch.");
    }
    std::puts("input mode identifiers keep the menu bar mode and the Chinese/English/Japanese/Korean state in step");
    return 0;
}
