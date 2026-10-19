/** 表情面板搜索框与剪贴板记录共用字段配色，明暗由面板根属性控制。 */
export const panelField =
  "border border-[#3a3a44] bg-[#2b2b33] text-[#f5f5f7] group-data-[panel-theme=light]:border-[#c8ccd5] group-data-[panel-theme=light]:bg-white group-data-[panel-theme=light]:text-[#202124]";

export const clipboardRow = `w-full flex-1 min-w-0 overflow-hidden rounded-lg px-3.5 py-3 text-left font-[inherit] text-ellipsis whitespace-nowrap ${panelField} hover:border-[#5fbf84] hover:bg-[#303038] focus-visible:border-[#5fbf84] focus-visible:bg-[#303038] group-data-[panel-theme=light]:hover:border-[#2c7a4b] group-data-[panel-theme=light]:hover:bg-[#eceef3] group-data-[panel-theme=light]:focus-visible:border-[#2c7a4b] group-data-[panel-theme=light]:focus-visible:bg-[#eceef3]`;

export const clipboardSideButton = `shrink-0 grow-0 basis-auto rounded-lg p-2 font-[inherit] ${panelField} hover:border-[#5fbf84] focus-visible:border-[#5fbf84] group-data-[panel-theme=light]:hover:border-[#2c7a4b] group-data-[panel-theme=light]:focus-visible:border-[#2c7a4b]`;
