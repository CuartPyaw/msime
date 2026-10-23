#pragma once
#import "TextClient.h"

/// 只从当前文档读取边界：选区、未知位置和不能确定起点的标记文本都不猜测。
static inline uint32_t MSIMECrossCommitSpacingPreceding(id<MSIMETextClient> client, BOOL hasMarkedText) {
    if (![client respondsToSelector:@selector(selectedRange)] ||
        ![client respondsToSelector:@selector(attributedSubstringFromRange:)]) return 0;
    @try {
        const NSRange selected = [client selectedRange];
        if (selected.location == NSNotFound || selected.length) return 0;
        NSUInteger boundary = selected.location;
        if (hasMarkedText) {
            if (![client respondsToSelector:@selector(markedRange)]) return 0;
            const NSRange marked = [client markedRange];
            if (marked.location == NSNotFound || selected.location < marked.location ||
                selected.location - marked.location > marked.length) return 0;
            boundary = marked.location;
        }
        return MSIMETextClientUnicodeScalarBeforeIndex(client, boundary);
    } @catch (NSException *exception) {
        (void)exception;
        return 0;
    }
}

/// NSString 的首个完整 Unicode 标量，扩展区汉字不能拆成两个 UTF-16 单位。
static inline uint32_t MSIMECrossCommitSpacingFirstScalar(NSString *text) {
    if (!text.length) return 0;
    const unichar first = [text characterAtIndex:0];
    if (CFStringIsSurrogateHighCharacter(first)) {
        if (text.length < 2) return 0;
        const unichar second = [text characterAtIndex:1];
        return CFStringIsSurrogateLowCharacter(second)
            ? CFStringGetLongCharacterForSurrogatePair(first, second) : 0;
    }
    return CFStringIsSurrogateLowCharacter(first) ? 0 : first;
}
