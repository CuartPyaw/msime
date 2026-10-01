#pragma once
#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#import <Foundation/Foundation.h>

using MSIMEInputSourceRegistrar = OSStatus (*)(CFURLRef);
using MSIMEInputSourceLister = CFArrayRef (*)(CFDictionaryRef, Boolean);
using MSIMEInputSourcePropertyGetter = void *(*)(TISInputSourceRef, CFStringRef);
using MSIMEInputSourceEnabler = OSStatus (*)(TISInputSourceRef);
using MSIMEInputSourceCopier = TISInputSourceRef (*)(void);

@interface MSIMEInputSourceMonitor : NSObject
- (instancetype)initWithCenter:(NSNotificationCenter *)center bundleIdentifier:(NSString *)identifier
                    copySource:(MSIMEInputSourceCopier)copier propertyGetter:(MSIMEInputSourcePropertyGetter)getter
                    switchedAway:(void (^)(void))action;
- (void)stop;
@end

bool MSIMEShouldRegisterInputSource(int argc, const char *argv[]);
OSStatus MSIMERegisterInputSource(NSURL *bundleURL, MSIMEInputSourceRegistrar registrar);
OSStatus MSIMERegisterAndEnableInputSources(NSURL *bundleURL, NSString *bundleIdentifier,
                                            MSIMEInputSourceRegistrar registrar,
                                            MSIMEInputSourceLister lister,
                                            MSIMEInputSourcePropertyGetter propertyGetter,
                                            MSIMEInputSourceEnabler enabler);
/// Enables each of the bundle's input modes that `offered` does not name yet, once, and returns `offered` with them added; the caller persists it. An update that only replaces the bundle does not re-register it, which leaves the modes it added off with no entry in System Settings' add dialog to turn them on, since that dialog does not list a third-party input method's modes. A mode already recorded is left alone, so one the user removed stays removed. A nil `offered` starts from the modes every earlier install enabled. The opt-in modes (MSIMEOptInInputModeIDs) are recorded without being enabled, and `disabler` turns one off the first time it is recorded if the system enabled it anyway.
NSArray<NSString *> *MSIMEEnableNewInputModes(NSString *bundleIdentifier, NSArray<NSString *> *offered,
                                              MSIMEInputSourceLister lister,
                                              MSIMEInputSourcePropertyGetter propertyGetter,
                                              MSIMEInputSourceEnabler enabler,
                                              MSIMEInputSourceEnabler disabler);
/// Enables the installed input source with this identifier, enabled or not. This is how an opt-in mode is turned on when the user picks its scheme: System Settings' add dialog does not list a third-party input method's modes.
OSStatus MSIMEEnableInputMode(NSString *identifier, MSIMEInputSourceLister lister, MSIMEInputSourceEnabler enabler);
/// Whether the input source with this identifier is enabled, so the system can select it.
BOOL MSIMEInputSourceIsEnabled(NSString *identifier);
/// Starts a separate non-activating helper instance of the current input method
/// to re-register its source. Completion is always delivered on the main thread.
void MSIMELaunchInputSourceReregistration(NSURL *bundleURL, NSWorkspace *workspace,
                                          void (^completion)(BOOL launched));
