/** 原生成功信封的 value 不能缺失；null 由各调用方按协议语义另行处理。 */
export class NativeReplyPolicy {
  static hasValue(value: unknown): boolean {
    return value !== undefined && value !== null;
  }
}
