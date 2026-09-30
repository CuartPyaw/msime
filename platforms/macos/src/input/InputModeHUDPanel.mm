#import "InputModeHUDPanel.h"

#import <cmath>

namespace {
constexpr CGFloat kPanelWidth = 100.0;
constexpr CGFloat kPanelHeight = 56.0;
constexpr CGFloat kLogoSide = 26.0;
constexpr CGFloat kContentSpacing = 6.0;
constexpr CGFloat kCornerRadius = 16.0;
constexpr CGFloat kBorderWidth = 1.0;
constexpr CGFloat kScreenMargin = 8.0;
constexpr CGFloat kCaretGap = 10.0;
constexpr NSTimeInterval kVisibleDuration = 0.6;
constexpr NSTimeInterval kFadeDuration = 0.18;

CGFloat Clamp(CGFloat value, CGFloat minimum, CGFloat maximum) {
    if (maximum < minimum) return minimum;
    return value < minimum ? minimum : (value > maximum ? maximum : value);
}
}

NSString *MSIMEInputModeHUDText(BOOL englishInputMode) { return englishInputMode ? @"英" : @"中"; }

BOOL MSIMEInputModeHUDUsableCaretRect(NSRect caretRect) {
    return std::isfinite(NSMinX(caretRect)) && std::isfinite(NSMinY(caretRect)) &&
           std::isfinite(NSMaxX(caretRect)) && std::isfinite(NSMaxY(caretRect)) && NSHeight(caretRect) > 0.0;
}

NSRect MSIMEInputModeHUDFrame(NSRect caretRect, NSSize panelSize, NSRect visibleFrame) {
    const CGFloat minimumX = NSMinX(visibleFrame) + kScreenMargin;
    const CGFloat maximumX = NSMaxX(visibleFrame) - kScreenMargin - panelSize.width;
    const CGFloat minimumY = NSMinY(visibleFrame) + kScreenMargin;
    const CGFloat maximumY = NSMaxY(visibleFrame) - kScreenMargin - panelSize.height;
    if (!MSIMEInputModeHUDUsableCaretRect(caretRect)) {
        const CGFloat centeredX = NSMidX(visibleFrame) - panelSize.width / 2.0;
        const CGFloat lowerThirdY = NSMinY(visibleFrame) + NSHeight(visibleFrame) / 4.0;
        return NSMakeRect(Clamp(centeredX, minimumX, maximumX), Clamp(lowerThirdY, minimumY, maximumY), panelSize.width, panelSize.height);
    }
    const CGFloat x = Clamp(NSMidX(caretRect) - panelSize.width / 2.0, minimumX, maximumX);
    const CGFloat belowY = NSMinY(caretRect) - kCaretGap - panelSize.height;
    const CGFloat preferredY = belowY >= minimumY ? belowY : NSMaxY(caretRect) + kCaretGap;
    return NSMakeRect(x, Clamp(preferredY, minimumY, maximumY), panelSize.width, panelSize.height);
}

@implementation MSIMEInputModeHUDPanel {
    NSTextField *_label;
    NSImageView *_logoView;
    NSTimer *_dismissTimer;
    NSColor *_surfaceColor;
    NSColor *_borderColor;
    NSColor *_textColor;
}

+ (instancetype)sharedPanel {
    static MSIMEInputModeHUDPanel *panel;
    static dispatch_once_t once;
    dispatch_once(&once, ^{ panel = [[self alloc] init]; });
    return panel;
}

- (instancetype)init {
    self = [super initWithContentRect:NSMakeRect(0, 0, kPanelWidth, kPanelHeight)
                              styleMask:NSWindowStyleMaskBorderless | NSWindowStyleMaskNonactivatingPanel
                                backing:NSBackingStoreBuffered defer:YES];
    if (!self) return nil;
    self.floatingPanel = YES;
    self.level = NSPopUpMenuWindowLevel;
    self.becomesKeyOnlyIfNeeded = YES;
    self.hidesOnDeactivate = NO;
    self.opaque = NO;
    self.backgroundColor = NSColor.clearColor;
    self.hasShadow = YES;
    self.ignoresMouseEvents = YES;
    self.collectionBehavior = NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary |
                              NSWindowCollectionBehaviorTransient | NSWindowCollectionBehaviorIgnoresCycle;
    self.animationBehavior = NSWindowAnimationBehaviorNone;

    NSView *background = [[NSView alloc] initWithFrame:NSMakeRect(0, 0, kPanelWidth, kPanelHeight)];
    background.wantsLayer = YES;
    background.layer.cornerRadius = kCornerRadius;
    background.layer.masksToBounds = YES;
    background.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    // The full-colour brand mark the floating toolbar leads with, not the monochrome menu bar template.
    NSString *logoPath = [[NSBundle bundleForClass:self.class] pathForResource:@"MSIMEClientInputMethod" ofType:@"icns"];
    NSImage *logo = logoPath ? [[NSImage alloc] initWithContentsOfFile:logoPath] : nil;
    _logoView = [NSImageView imageViewWithImage:logo ?: [[NSImage alloc] initWithSize:NSZeroSize]];
    _logoView.hidden = logo == nil;
    _logoView.imageScaling = NSImageScaleProportionallyUpOrDown;
    _logoView.translatesAutoresizingMaskIntoConstraints = NO;
    _label = [NSTextField labelWithString:@""];
    _label.alignment = NSTextAlignmentCenter;
    _label.font = [NSFont systemFontOfSize:30.0 weight:NSFontWeightSemibold];
    _label.translatesAutoresizingMaskIntoConstraints = NO;
    NSStackView *content = [NSStackView stackViewWithViews:logo ? @[_logoView, _label] : @[_label]];
    content.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    content.alignment = NSLayoutAttributeCenterY;
    content.spacing = kContentSpacing;
    content.translatesAutoresizingMaskIntoConstraints = NO;
    [background addSubview:content];
    [NSLayoutConstraint activateConstraints:@[
        [content.centerXAnchor constraintEqualToAnchor:background.centerXAnchor],
        [content.centerYAnchor constraintEqualToAnchor:background.centerYAnchor],
        [_logoView.widthAnchor constraintEqualToConstant:kLogoSide],
        [_logoView.heightAnchor constraintEqualToConstant:kLogoSide],
    ]];
    self.contentView = background;
    [self applyThemeColors];
    return self;
}

- (void)setSurfaceColor:(NSColor *)surface borderColor:(NSColor *)border textColor:(NSColor *)text {
    _surfaceColor = [surface copy];
    _borderColor = [border copy];
    _textColor = [text copy];
    [self applyThemeColors];
}

- (void)applyThemePreferences:(NSDictionary *)preferences {
    id surface = preferences[@"toolbar_theme"];
    id global = preferences[@"theme"];
    id resolved = ([surface isEqual:@"dark"] || [surface isEqual:@"light"]) ? surface : global;
    if ([resolved isEqual:@"light"]) self.appearance = [NSAppearance appearanceNamed:NSAppearanceNameAqua];
    else if (resolved == nil || [resolved isEqual:@"system"]) self.appearance = nil;
    else self.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
    [self applyThemeColors];
}

- (NSColor *)surfaceColor { return _surfaceColor ?: NSColor.windowBackgroundColor; }
- (NSColor *)borderColor { return _borderColor ?: NSColor.separatorColor; }
- (NSColor *)textColor { return _textColor ?: NSColor.labelColor; }

// Layer colours are resolved once, so they are taken again in the panel's current appearance each time it is shown.
- (void)applyThemeColors {
    [self.effectiveAppearance performAsCurrentDrawingAppearance:^{
        self.contentView.layer.backgroundColor = self.surfaceColor.CGColor;
        self.contentView.layer.borderColor = self.borderColor.CGColor;
        self.contentView.layer.borderWidth = kBorderWidth;
        self->_label.textColor = self.textColor;
    }];
}

- (BOOL)showsLogo { return !_logoView.hidden; }
- (NSString *)displayedText { return self.isVisible ? _label.stringValue : nil; }

- (void)showEnglishInputMode:(BOOL)englishInputMode nearCaretRect:(NSRect)caretRect {
    _label.stringValue = MSIMEInputModeHUDText(englishInputMode);
    NSAccessibilityPostNotificationWithUserInfo(self, NSAccessibilityAnnouncementRequestedNotification,
                                                 @{NSAccessibilityAnnouncementKey : englishInputMode ? @"英文输入" : @"中文输入"});
    NSScreen *screen = NSScreen.mainScreen;
    for (NSScreen *candidate in NSScreen.screens) {
        if (NSPointInRect(NSMakePoint(NSMidX(caretRect), NSMidY(caretRect)), candidate.frame)) { screen = candidate; break; }
    }
    NSRect visible = screen ? screen.visibleFrame : NSMakeRect(0, 0, 1440, 900);
    [self setFrame:MSIMEInputModeHUDFrame(caretRect, NSMakeSize(kPanelWidth, kPanelHeight), visible) display:YES];
    [self applyThemeColors];
    [_dismissTimer invalidate];
    self.alphaValue = 1.0;
    [self orderFrontRegardless];
    __weak MSIMEInputModeHUDPanel *weakSelf = self;
    _dismissTimer = [NSTimer scheduledTimerWithTimeInterval:kVisibleDuration repeats:NO block:^(NSTimer *timer) {
        (void)timer;
        [weakSelf fadeOut];
    }];
}

- (void)fadeOut {
    [NSAnimationContext runAnimationGroup:^(NSAnimationContext *context) {
        context.duration = kFadeDuration;
        self.animator.alphaValue = 0.0;
    } completionHandler:^{
        if (self.alphaValue <= 0.01) [self orderOut:nil];
    }];
}
@end
