#import <Foundation/Foundation.h>

/// A single sequential batch for the visible candidate page (at most nine items).
/// All methods and completion run on the main thread. Retain until completion.
@interface MSIMECustomTranslationBatch : NSObject
/// NiuTrans plan items use the shared direction plan. Sign immediately before each request.
- (instancetype)initWithNiuTransItems:(NSArray<NSDictionary *> *)items config:(NSDictionary *)config
                        configuration:(NSURLSessionConfiguration *)configuration
                           completion:(void (^)(NSArray<NSDictionary *> *translations))completion;
/// Items are {text, request}, where request is a shared custom HTTP descriptor.
/// Copies the input. Completion receives successful {text, translation} results.
- (instancetype)initWithItems:(NSArray<NSDictionary *> *)items
                configuration:(NSURLSessionConfiguration *)configuration
                   completion:(void (^)(NSArray<NSDictionary *> *translations))completion;
/// Tencent plan items are {text, key, source_language, target_language}.
/// Groups matching directions; signs each group immediately before transport.
/// Copies inputs and retains no credentials after cancellation/completion.
- (instancetype)initWithTencentItems:(NSArray<NSDictionary *> *)items
                               config:(NSDictionary *)config
                        configuration:(NSURLSessionConfiguration *)configuration
                           completion:(void (^)(NSArray<NSDictionary *> *translations))completion;
/// AI items are {text, request}; each request is a shared AI HTTP descriptor.
- (instancetype)initWithAIItems:(NSArray<NSDictionary *> *)items
                   configuration:(NSURLSessionConfiguration *)configuration
                      completion:(void (^)(NSArray<NSDictionary *> *translations))completion;
/// Called on the main thread once per response the batch handles, before completion. `answeredTexts` lists the item texts that response answered, whether or not a translation came back for each; items never sent, and items whose transport failed, are absent, so a caller can negative-cache exactly the answered ones. A response that arrives after the deadline is still reported here, although it is left out of completion's results. Set before -start.
@property(nonatomic, copy) void (^onReply)(NSArray<NSDictionary *> *results, NSArray<NSString *> *answeredTexts);
/// Single-use. A six-second whole-batch deadline returns completed partial results.
- (void)start;
/// Lets the request already in flight land through onReply but starts no further item and never calls the original completion; `completion` runs instead once the batch ends by reply or deadline. The six-second deadline still applies. Returns NO, and cancels, when nothing is in flight.
- (BOOL)detachWithCompletion:(void (^)(void))completion;
/// Suppresses completion and onReply, aborts transport and releases credential-bearing inputs.
- (void)cancel;
@end
