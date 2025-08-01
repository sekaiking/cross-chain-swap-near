const getStorageKey = (accountId: string, tokenAddress: string): string => {
  return `near-token-approval-${accountId}-${tokenAddress}`;
};

export const checkTokenApprovalStatus = (accountId: string, tokenAddress: string): boolean => {
  if (typeof window === "undefined") return false;
  const key = getStorageKey(accountId, tokenAddress);
  return localStorage.getItem(key) === 'true';
};

export const setTokenApprovalStatus = (accountId: string, tokenAddress: string): void => {
  if (typeof window === "undefined") return;
  const key = getStorageKey(accountId, tokenAddress);
  localStorage.setItem(key, 'true');
};
