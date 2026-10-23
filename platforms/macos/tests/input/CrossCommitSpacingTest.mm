#import "../../src/input/InputController.mm"
#import "../settings/TestPreferenceSuite.h"
#include <cassert>
#include <sqlite3.h>

@interface SpacingClient : NSObject <MSIMETextClient>
@property(nonatomic, copy) NSString *document;
@property(nonatomic) NSRange selection;
@property(nonatomic) NSRange marking;
@property(nonatomic) BOOL unreadable;
@end
@implementation SpacingClient
- (NSString *)bundleIdentifier { return @"app.example.Writer"; }
- (NSRange)selectedRange { return self.selection; }
- (NSRange)markedRange { return self.marking; }
- (NSAttributedString *)attributedSubstringFromRange:(NSRange)range {
    if (self.unreadable || range.location > self.document.length || range.length > self.document.length - range.location) return nil;
    return [[NSAttributedString alloc] initWithString:[self.document substringWithRange:range]];
}
- (void)insertText:(id)value replacementRange:(NSRange)range {
    NSString *text = [value isKindOfClass:NSAttributedString.class] ? [value string] : value;
    NSRange target = range.location != NSNotFound ? range : self.marking.location != NSNotFound ? self.marking : self.selection;
    self.document = [self.document stringByReplacingCharactersInRange:target withString:text];
    self.selection = NSMakeRange(target.location + text.length, 0);
    self.marking = NSMakeRange(NSNotFound, 0);
}
- (void)setMarkedText:(id)value selectionRange:(NSRange)selection replacementRange:(NSRange)replacement {
    NSString *text = [value isKindOfClass:NSAttributedString.class] ? [value string] : value;
    NSRange target = replacement.location != NSNotFound ? replacement : self.marking.location != NSNotFound ? self.marking : self.selection;
    self.document = [self.document stringByReplacingCharactersInRange:target withString:text ?: @""];
    self.marking = text.length ? NSMakeRange(target.location, text.length) : NSMakeRange(NSNotFound, 0);
    self.selection = NSMakeRange(target.location + selection.location, selection.length);
}
@end

@interface SpacingController : MSIMEInputController
@property(nonatomic) BOOL secure;
@end
@implementation SpacingController
- (void)ensureAppearance {}
- (void)prepareSession {}
- (BOOL)secureEventInputActive { return self.secure; }
// 直通分支专门交给外层处理；真实候选上屏另经 apply: 验证。
- (BOOL)handleKeyEvent:(NSEvent *)event client:(id)sender { (void)event; (void)sender; return NO; }
- (void)recordKeyPress:(NSEvent *)event client:(id)sender { (void)event; (void)sender; }
- (void)recordPassthroughKey:(NSEvent *)event client:(id)sender { (void)event; (void)sender; }
- (void)reportKeyToLifecycle:(NSEvent *)event { (void)event; }
- (void)renderCandidates {}
- (void)synchronizeCandidateServices {}
- (void)refreshFloatingToolbarState {}
@end

static SpacingClient *Client(NSString *document) {
    SpacingClient *client = [SpacingClient new];
    client.document = document;
    client.selection = NSMakeRange(document.length, 0);
    client.marking = NSMakeRange(NSNotFound, 0);
    return client;
}
static NSEvent *Key(NSString *text, NSEventModifierFlags flags = 0) {
    return [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint modifierFlags:flags timestamp:0
        windowNumber:0 context:nil characters:text charactersIgnoringModifiers:text isARepeat:NO keyCode:0];
}

int main() {
    @autoreleasepool {
        [NSApplication sharedApplication];
        char *defaults = msime_client_default_preferences();
        NSDictionary *reply = [NSJSONSerialization JSONObjectWithData:[[NSString stringWithUTF8String:defaults] dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil];
        msime_client_string_free(defaults);
        NSMutableDictionary *preferences = [reply[@"value"] mutableCopy];
        preferences[@"default_ime_mode"] = @"chinese";
        preferences[@"cross_commit_spacing"] = @YES;
        preferences[@"learning"] = @NO;
        NSString *root = [NSTemporaryDirectory() stringByAppendingPathComponent:NSUUID.UUID.UUIDString];
        NSMutableDictionary *options = [@{@"api_version": @1, @"preferences": preferences} mutableCopy];
        for (NSString *name in @[@"resources", @"user_data", @"cache", @"dictionaries"]) {
            options[name] = [root stringByAppendingPathComponent:name];
            assert([NSFileManager.defaultManager createDirectoryAtPath:options[name] withIntermediateDirectories:YES attributes:nil error:nil]);
        }
        sqlite3 *database = nullptr;
        assert(sqlite3_open([[options[@"dictionaries"] stringByAppendingPathComponent:@"msime-pinyin.db"] fileSystemRepresentation], &database) == SQLITE_OK);
        assert(sqlite3_exec(database, "CREATE TABLE tbl_1_j(key TEXT,jp TEXT,value TEXT,weight INTEGER); INSERT INTO tbl_1_j VALUES('jia','j','甲',100);", nullptr, nullptr, nullptr) == SQLITE_OK);
        assert(sqlite3_close(database) == SQLITE_OK);
        NSError *error = nil;
        MSIMEClientSession *session = [[MSIMEClientSession alloc] initWithOptions:options error:&error];
        assert(session && !error);
        assert([session setFocused:YES error:&error] && !error);
        NSString *suite = [@"msime.cross-commit-spacing." stringByAppendingString:NSUUID.UUID.UUIDString];
        NSUserDefaults *defaultsSuite = [[NSUserDefaults alloc] initWithSuiteName:suite];
        MSIMEAppearancePreferences *appearance = [[MSIMEAppearancePreferences alloc] initWithDefaults:defaultsSuite];
        appearance.floatingToolbarEnabled = NO;
        [appearance applySharedInputPreferences:preferences];
        SpacingController *controller = [SpacingController alloc];
        [controller setValue:appearance forKey:@"appearance"];
        [controller setValue:session forKey:@"session"];
        [controller setValue:@{@"scheme": @0, @"editing_text": @"", @"candidates": @[]} forKey:@"view"];
        SpacingClient *client = Client(@"甲");
        [controller setValue:client forKey:@"activeClient"];
        assert([controller handleEvent:Key(@"A") client:client]);
        assert([client.document isEqual:@"甲 A"]);
        assert(![controller handleEvent:Key(@"B") client:client]);
        assert(![controller handleEvent:Key(@"A", NSEventModifierFlagCommand) client:client]);
        client.unreadable = YES;
        assert([[controller crossCommitSpacedText:@"乙" client:client] isEqual:@"乙"]);
        client.unreadable = NO;
        client.selection = NSMakeRange(0, 1);
        assert([[controller crossCommitSpacedText:@"A" client:client] isEqual:@"A"]);
        client = Client(@"𠀀");
        assert([[controller crossCommitSpacedText:@"9" client:client] isEqual:@" 9"]);
        client = Client(@"A ");
        assert([[controller crossCommitSpacedText:@"乙" client:client] isEqual:@"乙"]);
        client = Client(@"A拼音");
        client.marking = NSMakeRange(1, 2);
        [controller setValue:@YES forKey:@"clientHasMarkedText"];
        assert([[controller crossCommitSpacedText:@"乙" client:client] isEqual:@" 乙"]);
        client.marking = NSMakeRange(NSNotFound, 0);
        assert([[controller crossCommitSpacedText:@"乙" client:client] isEqual:@"乙"]);
        [controller setValue:@NO forKey:@"clientHasMarkedText"];
        controller.secure = YES;
        assert([[controller crossCommitSpacedText:@"A" client:Client(@"甲")] isEqual:@"A"]);
        controller.secure = NO;
        // 候选提交经真实会话与控制器 apply:，避开文档里的预编辑后在英文右侧补空格。
        client = Client(@"A");
        [controller setValue:client forKey:@"activeClient"];
        for (uint8_t character : {'j', 'i', 'a'})
            [controller apply:[session typeASCII:character shift:NO error:nil]];
        [controller apply:[session command:MSIME_COMMIT_CANDIDATE error:nil]];
        assert([client.document isEqual:@"A 甲"]);
        client = Client(@"甲");
        [controller setValue:client forKey:@"activeClient"];
        [controller apply:@{@"commit": @"AB", @"commit_context": @{@"scheme": @0, @"local_mode": @"temporary_english"},
            @"view": [session viewWithError:nil]}];
        assert([client.document isEqual:@"甲 AB"]);
        client = Client(@"甲");
        [controller setValue:client forKey:@"activeClient"];
        [controller apply:@{@"commit": @"AB", @"commit_context": @{@"scheme": @0, @"local_mode": @"url"},
            @"view": [session viewWithError:nil]}];
        assert([client.document isEqual:@"甲AB"]);
        preferences[@"cross_commit_spacing_excluded_apps"] = @[@"APP.EXAMPLE.WRITER"];
        assert(([session updatePreferencesSnapshot:@{@"format_version": @1, @"revision": @1, @"preferences": preferences} error:nil]));
        assert([[controller crossCommitSpacedText:@"A" client:Client(@"甲")] isEqual:@"A"]);
        [controller setValue:nil forKey:@"session"];
        session = nil;
        assert([NSFileManager.defaultManager removeItemAtPath:root error:nil]);
        MSIMERemoveTestPreferenceSuite(defaultsSuite, suite);
    }
    return 0;
}
