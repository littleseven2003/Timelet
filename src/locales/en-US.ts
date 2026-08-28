// 预留的英文语言包，尚未启用（界面文案以 zh-CN 为准）
export default {
  app: {
    name: 'Timelet',
    tagline: 'Keep important days at hand',
  },
  panel: {
    emptyTitle: 'No countdown yet',
    emptyHint: 'Open the settings window to add your first entry',
  },
  config: {
    nav: {
      entries: 'Entries',
      settings: 'Settings',
      about: 'About',
    },
    settingsPlaceholder: 'Settings will be available in a later version',
    aboutPlaceholder: 'About info will be available in a later version',
    addEntry: 'Add Entry',
    editEntry: 'Edit Entry',
    emptyList: 'No entries yet. Click the button above to add one',
    fieldName: 'Name',
    namePlaceholder: 'e.g. Project launch',
    fieldType: 'Type',
    typeCountdown: 'Countdown',
    typeElapsed: 'Elapsed',
    targetDate: 'Target date',
    startDate: 'Start date',
    fieldColor: 'Color',
    fieldPinned: 'Pinned',
    pinnedTag: 'Pinned',
    save: 'Save',
    cancel: 'Cancel',
    edit: 'Edit',
    delete: 'Delete',
    confirmDelete: 'Confirm',
  },
} as const;
