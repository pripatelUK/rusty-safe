export const MM_PARITY_SCENARIOS = [
  {
    scenarioId: "MM-PARITY-001",
    parityIds: ["PARITY-TX-01"],
    method: "eth_requestAccounts",
    title: "connect via eth_requestAccounts",
    timeoutMs: 45000,
    requiresAnvil: false,
  },
  {
    scenarioId: "MM-PARITY-002",
    parityIds: ["PARITY-MSG-01"],
    method: "personal_sign",
    title: "message signing via personal_sign",
    timeoutMs: 45000,
    requiresAnvil: false,
  },
  {
    scenarioId: "MM-PARITY-003",
    parityIds: ["PARITY-MSG-01"],
    method: "eth_signTypedData_v4",
    title: "typed data signing via eth_signTypedData_v4",
    timeoutMs: 45000,
    requiresAnvil: false,
  },
  {
    scenarioId: "MM-PARITY-004",
    parityIds: ["PARITY-TX-02"],
    method: "eth_sendTransaction",
    title: "transaction send via eth_sendTransaction",
    timeoutMs: 60000,
    requiresAnvil: true,
  },
];

