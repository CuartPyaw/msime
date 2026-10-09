#import "TestPreferenceSuite.h"
#import "../../src/settings/AppearancePreferences.h"
#import "../../src/cloud/CloudAppearanceSettings.h"
#include <cassert>

// 四码唯一自动上屏换缺省值（云契约的 @NO -> @YES）时的迁移：接入共享偏好之前 macOS 从不兑现这个键，
// 所以 defaults 里和云快照里的 false 都是历史缺省，不是用户选择。只有用户在原生面板里真正选过一次之后，
// 存下来的值才作数——否则升级后（或下一次套用旧云快照后）第四键会莫名其妙不再上屏。
static NSString *const WubiAutoCommitKey = @"MSIMEClientWubiAutoCommitUnique";
static NSString *const WubiAutoCommitAdoptedKey = @"MSIMEClientWubiAutoCommitUniqueAdopted";

int main(void)
{
    @autoreleasepool {
        NSString *suite = [@"MSIME.WubiAutoCommit." stringByAppendingString:NSUUID.UUID.UUIDString];
        NSUserDefaults *defaults = [[NSUserDefaults alloc] initWithSuiteName:suite];

        // 升级场景：defaults 里留着旧缺省写下的 NO，用户从没选过 -> 读作开。
        [defaults setBool:NO forKey:WubiAutoCommitKey];
        MSIMEAppearancePreferences *preferences = [[MSIMEAppearancePreferences alloc] initWithDefaults:defaults];
        assert(preferences.wubiAutoCommitUnique);
        // 导出取生效值，所以共享文档和云快照都拿到 true，而不是 defaults 里的 false。
        assert([[preferences sharedPreferencesByMerging:@{}][@"wubi_auto_commit_unique"] isEqual:@YES]);
        assert([[preferences cloudSettingsSnapshot][@"platform.macos.wubi_auto_commit_unique"] isEqual:@YES]);

        // 旧账号同步下来的 false 也是历史缺省：套用后仍读作开，且没有把用户记成已经选过。
        NSMutableDictionary *cloud = [[preferences cloudSettingsSnapshot] mutableCopy];
        cloud[@"platform.macos.wubi_auto_commit_unique"] = @NO;
        assert([preferences applyCloudSettingsSnapshot:cloud]);
        assert(preferences.wubiAutoCommitUnique);
        assert([defaults objectForKey:WubiAutoCommitAdoptedKey] == nil);

        // 用户在原生面板里第一次选关：这才是新语义下的真实选择，此后存下来的值作数。
        preferences.wubiAutoCommitUnique = NO;
        assert(!preferences.wubiAutoCommitUnique);
        assert([[defaults objectForKey:WubiAutoCommitAdoptedKey] boolValue]);
        assert([[preferences sharedPreferencesByMerging:@{}][@"wubi_auto_commit_unique"] isEqual:@NO]);

        // 选择持久化到新建的对象上。
        MSIMEAppearancePreferences *fresh = [[MSIMEAppearancePreferences alloc] initWithDefaults:defaults];
        assert(!fresh.wubiAutoCommitUnique);
        // 此后云端来的 false 与已选择的值一致，自然继续生效。
        assert([fresh applyCloudSettingsSnapshot:cloud]);
        assert(!fresh.wubiAutoCommitUnique);

        MSIMERemoveTestPreferenceSuite(defaults, suite);
    }
    return 0;
}
