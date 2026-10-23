#import "../../src/input/CrossCommitSpacing.h"
#include <cassert>

@interface SpacingContextClient : NSObject <MSIMETextClient>
@property(nonatomic, copy) NSString *document;
@property(nonatomic) NSRange selection;
@property(nonatomic) NSRange marking;
@property(nonatomic) BOOL unreadable;
@end
@implementation SpacingContextClient
- (NSRange)selectedRange { return self.selection; }
- (NSRange)markedRange { return self.marking; }
- (NSAttributedString *)attributedSubstringFromRange:(NSRange)range {
    if (self.unreadable) return nil;
    return [[NSAttributedString alloc] initWithString:[self.document substringWithRange:range]];
}
- (void)insertText:(id)text replacementRange:(NSRange)range { (void)text; (void)range; }
- (void)setMarkedText:(id)text selectionRange:(NSRange)selection replacementRange:(NSRange)replacement {
    (void)text; (void)selection; (void)replacement;
}
@end

int main() {
    @autoreleasepool {
        SpacingContextClient *client = [SpacingContextClient new];
        client.document = @"𠀀拼音";
        client.marking = NSMakeRange(2, 2);
        client.selection = NSMakeRange(4, 0);
        assert(MSIMECrossCommitSpacingPreceding(client, YES) == 0x20000);
        assert(MSIMECrossCommitSpacingPreceding(client, NO) == 0x97f3);
        client.selection = NSMakeRange(3, 1);
        assert(MSIMECrossCommitSpacingPreceding(client, YES) == 0);
        client.selection = NSMakeRange(4, 0);
        client.marking = NSMakeRange(NSNotFound, 0);
        assert(MSIMECrossCommitSpacingPreceding(client, YES) == 0);
        client.marking = NSMakeRange(0, 4);
        assert(MSIMECrossCommitSpacingPreceding(client, YES) == 0);
        client.unreadable = YES;
        assert(MSIMECrossCommitSpacingPreceding(client, NO) == 0);
        client.unreadable = NO;
        client.selection = NSMakeRange(99, 0);
        assert(MSIMECrossCommitSpacingPreceding(client, NO) == 0);
        assert(MSIMECrossCommitSpacingFirstScalar(@"𠀀A") == 0x20000);
        assert(MSIMECrossCommitSpacingFirstScalar(@" A") == ' ');
        assert(MSIMECrossCommitSpacingFirstScalar(@"") == 0);
        const unichar invalid = 0xd800;
        assert(MSIMECrossCommitSpacingFirstScalar([NSString stringWithCharacters:&invalid length:1]) == 0);
    }
    return 0;
}
