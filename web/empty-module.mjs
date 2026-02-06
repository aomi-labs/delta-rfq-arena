// Empty module stub for optional dependencies like 'porto'
// This prevents build errors when optional peer dependencies aren't installed

export const Porto = {
  create: () => Promise.reject(new Error("Porto connector not available")),
};

export default {
  Porto: {
    create: () => Promise.reject(new Error("Porto connector not available")),
  },
};
