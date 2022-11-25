import { ethers, network } from "hardhat";
import { expect } from "chai";
import { XCVM } from "xcvm-typescript-sdk";

const protobuf = require("protobufjs");

describe("Interpreter", function () {
  let router: any;
  let interpreter: any;
  let owner: any;
  let user1: any;
  let user2: any;
  let accounts: any;
  let interpreterAddress: any;
  let erc20: any;
  let Interpreter: any;
  beforeEach(async function () {
    accounts = await ethers.getSigners();
    [owner, user1, user2] = accounts;
    Interpreter = await ethers.getContractFactory("Interpreter");
    const Router = await ethers.getContractFactory("Router");
    router = await Router.deploy();
    //register owner as the bridge
    await router.registerBridge(owner.address, 1, 1);

    await router.createInterpreter({
      networkId: 1,
      account: owner.address,
    });
    interpreterAddress = await router.userInterpreter(1, owner.address);
    const ERC20Mock = await ethers.getContractFactory("ERC20Mock");
    erc20 = await ERC20Mock.deploy("test", "test", interpreterAddress, ethers.utils.parseEther("10000000000000000"));
    await router.registerAsset(erc20.address, 1);
  });

  describe("interpreter with protobuf", function () {
    it("test program using sdk: transfer unit to relayer", async function () {
      let xcvm = new XCVM();
      let data = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createTransfer(xcvm.createRelayer(), [
              xcvm.createAsset(
                xcvm.createAssetId(1),
                xcvm.createBalance(
                  // 1.5
                  xcvm.createUnit(1, xcvm.createRatio("1000", "2000"))
                )
              ),
            ])
          ),
        ])
      );
      await router.runProgram({ networkId: 1, account: owner.address }, xcvm.encodeMessage(data), [], []);
      // 1.5 units
      expect((await erc20.balanceOf(owner.address)).toString()).to.be.equal("1500000000000000000");
    });

    it("test program using sdk: transfer unit", async function () {
      let xcvm = new XCVM();
      let data = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createTransfer(xcvm.createAccount(owner.address), [
              xcvm.createAsset(
                xcvm.createAssetId(1),
                xcvm.createBalance(
                  // 1.5
                  xcvm.createUnit(1, xcvm.createRatio("1000", "2000"))
                )
              ),
            ])
          ),
        ])
      );
      await router.runProgram({ networkId: 1, account: owner.address }, xcvm.encodeMessage(data), [], []);
      // 1.5 units
      expect((await erc20.balanceOf(owner.address)).toString()).to.be.equal("1500000000000000000");
    });

    it("test program using sdk: transfer ratio", async function () {
      let xcvm = new XCVM();
      let data = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createTransfer(xcvm.createAccount(owner.address), [
              xcvm.createAsset(
                xcvm.createAssetId(1),
                xcvm.createBalance(xcvm.createRatio("1000000000000000000000000000", "2000000000000000000000000000"))
              ),
            ])
          ),
        ])
      );
      await router.runProgram({ networkId: 1, account: owner.address }, xcvm.encodeMessage(data), [], []);
      expect((await erc20.balanceOf(owner.address)).toString()).to.be.equal("5000000000000000000000000000000000");
    });

    it("test program using sdk: transfer absolute", async function () {
      let xcvm = new XCVM();
      let data = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createTransfer(xcvm.createAccount(owner.address), [
              xcvm.createAsset(
                xcvm.createAssetId(1),
                xcvm.createBalance(xcvm.createAbsolute("1000000000000000000000000000"))
              ),
            ])
          ),
        ])
      );
      let cc = xcvm.ProgramMessage.decode(xcvm.encodeMessage(data));
      await router.runProgram({ networkId: 1, account: owner.address }, xcvm.encodeMessage(data), [], []);
      expect((await erc20.balanceOf(owner.address)).toString()).to.be.equal("1000000000000000000000000000");
    });

    it("test call function using sdk", async function () {
      let functionSignature = erc20.interface.encodeFunctionData("transfer", [
        user1.address,
        ethers.utils.parseEther("100"),
      ]);
      const abiCoder = ethers.utils.defaultAbiCoder;
      const payload = ethers.utils.concat([
        ethers.utils.arrayify(abiCoder.encode(["address"], [erc20.address])),
        ethers.utils.arrayify(functionSignature),
      ]);
      let xcvm = new XCVM();
      let programMessage = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([xcvm.createInstruction(xcvm.createCall(payload, xcvm.createBindings([])))])
      );

      let encodedProgram = xcvm.encodeMessage(programMessage);
      await router.runProgram({ networkId: 1, account: owner.address }, encodedProgram, [], []);
      expect((await erc20.balanceOf(user1.address)).toString()).to.be.equal(ethers.utils.parseEther("100").toString());
    });

    it("test call function using sdk: call with late binding", async function () {
      const abiCoder = ethers.utils.defaultAbiCoder;

      let functionSignature = erc20.interface.getSighash("transfer(address,uint256)");
      // placeholder 1 and 2
      const payload = ethers.utils.concat([
        ethers.utils.arrayify("0x01"),
        ethers.utils.arrayify(functionSignature),
        abiCoder.encode(["address"], [user1.address]),
        ethers.utils.arrayify("0x02"),
      ]);
      let xcvm = new XCVM();
      let programMessage = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createCall(
              payload,
              xcvm.createBindings([
                xcvm.createBinding(0, xcvm.createBindingValue(xcvm.createAssetId(1))),
                // bindingValuePosition(1 byte) + function signature (4bytes) + address(32bytes, its encoded) = 37 => balanceValuePosition
                xcvm.createBinding(
                  37,
                  xcvm.createBindingValue(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio(1, 2)))
                ),
              ])
            )
          ),
        ])
      );

      let encodedProgram = xcvm.encodeMessage(programMessage);
      await router.runProgram({ networkId: 1, account: owner.address }, encodedProgram, [], []);
      expect((await erc20.balanceOf(user1.address)).toString()).to.be.equal(
        ethers.utils.parseEther("5000000000000000").toString()
      );
    });

    it("test spawn program using sdk", async function () {
      let xcvm = new XCVM();
      let programMessage = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createTransfer(xcvm.createAccount(owner.address), [
              xcvm.createAsset(xcvm.createAssetId(1), xcvm.createBalance(xcvm.createAbsolute("100"))),
            ])
          ),
        ])
      );

      let data = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createSpawn(xcvm.createNetwork(1), xcvm.createSalt("0x11"), 1, programMessage, [
              xcvm.createAsset(xcvm.createAssetId(1), xcvm.createBalance(xcvm.createAbsolute(200))),
            ])
          ),
        ])
      );

      await expect(router.runProgram({ networkId: 1, account: owner.address }, xcvm.encodeMessage(data), [], []))
        .to.emit(router, "Spawn")
        .withArgs(
          owner.address.toLowerCase(),
          1,
          1,
          "0x11",
          ethers.utils.hexlify(xcvm.encodeMessage(programMessage)),
          [erc20.address],
          [200]
        );
      expect((await erc20.balanceOf(owner.address)).toString()).to.be.equal("200");
    });

    it("test program with multiple instructions", async function () {
      const abiCoder = ethers.utils.defaultAbiCoder;

      let functionSignature = erc20.interface.getSighash("transfer(address,uint256)");
      // placeholder 1 and 2
      const payload = ethers.utils.concat([
        ethers.utils.arrayify("0x01"),
        ethers.utils.arrayify(functionSignature),
        abiCoder.encode(["address"], [user1.address]),
        ethers.utils.arrayify("0x02"),
      ]);
      let xcvm = new XCVM();
      let programMessage = xcvm.createProgram(
        "0x01",
        xcvm.createInstructions([
          xcvm.createInstruction(
            xcvm.createCall(
              payload,
              xcvm.createBindings([
                xcvm.createBinding(0, xcvm.createBindingValue(xcvm.createAssetId(1))),
                // bindingValuePosition(1 byte) + function signature (4bytes) + address(32bytes, its encoded) = 37 => balanceValuePosition
                xcvm.createBinding(
                  37,
                  xcvm.createBindingValue(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio(1, 2)))
                ),
              ])
            )
          ),
          xcvm.createInstruction(
            xcvm.createTransfer(xcvm.createAccount(owner.address), [
              xcvm.createAsset(
                xcvm.createAssetId(1),
                xcvm.createBalance(
                  // 1.5
                  xcvm.createUnit(1, xcvm.createRatio("1000", "2000"))
                )
              ),
            ])
          ),
        ])
      );

      let encodedProgram = xcvm.encodeMessage(programMessage);
      await router.runProgram({ networkId: 1, account: owner.address }, encodedProgram, [], []);
      expect((await erc20.balanceOf(user1.address)).toString()).to.be.equal(
        ethers.utils.parseEther("5000000000000000").toString()
      );
      expect((await erc20.balanceOf(owner.address)).toString()).to.be.equal(ethers.utils.parseEther("1.5").toString());
    });
  });

  it("test generating uint128", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.convertUint128("100")))).to.be.equal(
      await interpreter.generateUint128("100")
    );
  });

  it("test generating absolute", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createAbsolute("100")))).to.be.equal(
      await interpreter.generateAbsolute("100")
    );
  });

  it("test generating ratio", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createRatio("100", "200")))).to.be.equal(
      await interpreter.generateRatio("100", "200")
    );
  });

  it("test generating unit", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createUnit("100", xcvm.createRatio("100", "200"))))
    ).to.be.equal(await interpreter.generateUnit("100", await interpreter.generateRatio("100", "200")));
  });

  it("test generating balance by ratio", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createBalance(xcvm.createRatio("100", "200"))))).to.be.equal(
      await interpreter.generateBalanceByRatio(await interpreter.generateRatio("100", "200"))
    );
  });

  it("test generating balance by absolute", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createBalance(xcvm.createAbsolute("100"))))).to.be.equal(
      await interpreter.generateBalanceByAbsolute(await interpreter.generateAbsolute("100"))
    );
  });

  it("test generating balance by unit", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(xcvm.createBalance(xcvm.createUnit("100", xcvm.createRatio("100", "200"))))
      )
    ).to.be.equal(
      await interpreter.generateBalanceByUnit(
        await interpreter.generateUnit("100", await interpreter.generateRatio("100", "200"))
      )
    );
  });

  it("test generating account", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createAccount("0x1111")))).to.be.equal(
      await interpreter.generateAccount("0x1111")
    );
  });

  it("test generating assetId", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createAssetId(1))))
        .to.be.equal(await interpreter.generateAssetId("1"));
  });

  it("test generating asset", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(xcvm.createAsset(xcvm.createAssetId(1), xcvm.createBalance(xcvm.createRatio("100", "200"))))
      )
    ).to.be.equal(
      await interpreter.generateAsset(
        await interpreter.generateAssetId("1"),
        await interpreter.generateBalanceByRatio(await interpreter.generateRatio("100", "200"))
      )
    );
  });
  // self, relayer register
  it("test generating assetAmount ", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200")))
      )
    ).to.be.equal(
      await interpreter.generateAssetAmount(
        await interpreter.generateAssetId("1"),
        await interpreter.generateRatio("100", "200")
      )
    );
  });

  it("test generating bindingValue by assetId ", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createBindingValue(xcvm.createAssetId(1))))).to.be.equal(
      await interpreter.generateBindingValueByAssetId(await interpreter.generateAssetId("1"))
    );
  });

  it("test generating bindingValue assetAmount ", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createBindingValue(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200")))
        )
      )
    ).to.be.equal(
      await interpreter.generateBindingValueByAssetAmount(
        await interpreter.generateAssetAmount(
          await interpreter.generateAssetId("1"),
          await interpreter.generateRatio("100", "200")
        )
      )
    );
  });

  it("test generating binding", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createBinding(
            1,
            xcvm.createBindingValue(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200")))
          )
        )
      )
    ).to.be.equal(
      await interpreter.generateBinding(
        1,
        await interpreter.generateBindingValueByAssetAmount(
          await interpreter.generateAssetAmount(
            await interpreter.generateAssetId("1"),
            await interpreter.generateRatio("100", "200")
          )
        )
      )
    );
  });

  it("test generating bindings", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createBindings([
            xcvm.createBinding(
              1,
              xcvm.createBindingValue(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200")))
            ),
            xcvm.createBinding(2, xcvm.createBindingValue(xcvm.createAssetId(1))),
          ])
        )
      )
    ).to.be.equal(
      await interpreter.generateBindings([
        await interpreter.generateBinding(
          1,
          await interpreter.generateBindingValueByAssetAmount(
            await interpreter.generateAssetAmount(
              await interpreter.generateAssetId("1"),
              await interpreter.generateRatio("100", "200")
            )
          )
        ),
        await interpreter.generateBinding(
          2,
          await interpreter.generateBindingValueByAssetId(await interpreter.generateAssetId("1"))
        ),
      ])
    );
  });

  it("test generating transfer by account", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createTransfer(xcvm.createAccount(owner.address), [
            xcvm.createAsset(
              xcvm.createAssetId(1),
              xcvm.createBalance(xcvm.createUnit(1, xcvm.createRatio("100", "200")))
            ),
            xcvm.createAsset(xcvm.createAssetId(2), xcvm.createBalance(xcvm.createAbsolute(1))),
          ])
        )
      )
    ).to.be.equal(
      await interpreter.generateTransferByAccount(await interpreter.generateAccount(owner.address), [
        await interpreter.generateAsset(
          await interpreter.generateAssetId("1"),
          await interpreter.generateBalanceByUnit(
            await interpreter.generateUnit(1, await interpreter.generateRatio("100", "200"))
          )
        ),
        await interpreter.generateAsset(
          await interpreter.generateAssetId("2"),
          await interpreter.generateBalanceByAbsolute(await interpreter.generateAbsolute(1))
        ),
      ])
    );
  });

  it("test generating instruction by transfer", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createInstruction(
            xcvm.createTransfer(xcvm.createAccount(owner.address), [
              xcvm.createAsset(
                xcvm.createAssetId(1),
                xcvm.createBalance(xcvm.createUnit(1, xcvm.createRatio("100", "200")))
              ),
              xcvm.createAsset(xcvm.createAssetId(2), xcvm.createBalance(xcvm.createAbsolute(1))),
            ])
          )
        )
      )
    ).to.be.equal(
      await interpreter.generateInstructionByTransfer(
        await interpreter.generateTransferByAccount(await interpreter.generateAccount(owner.address), [
          await interpreter.generateAsset(
            await interpreter.generateAssetId("1"),
            await interpreter.generateBalanceByUnit(
              await interpreter.generateUnit(1, await interpreter.generateRatio("100", "200"))
            )
          ),
          await interpreter.generateAsset(
            await interpreter.generateAssetId("2"),
            await interpreter.generateBalanceByAbsolute(await interpreter.generateAbsolute(1))
          ),
        ])
      )
    );
  });

  it("test generating salt", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createSalt("0x1111")))).to.be.equal(
      await interpreter.generateSalt("0x1111")
    );
  });

  it("test generating network", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(ethers.utils.hexlify(xcvm.encodeMessage(xcvm.createNetwork(1)))).to.be.equal(
      await interpreter.generateNetwork(1)
    );
  });

  it("test generating spawn", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createSpawn(
            xcvm.createNetwork(1),
            xcvm.createSalt("0x11"),
            1,
            xcvm.createProgram(
              ethers.utils.hexlify("0x01"),
              xcvm.createInstructions([
                xcvm.createInstruction(
                  xcvm.createTransfer(xcvm.createAccount(owner.address), [
                    xcvm.createAsset(xcvm.createAssetId(1), xcvm.createBalance(xcvm.createAbsolute("100"))),
                  ])
                ),
              ])
            ),
            [xcvm.createAsset(xcvm.createAssetId(1), xcvm.createBalance(xcvm.createAbsolute(200)))]
          )
        )
      )
    ).to.be.equal(
      await interpreter.generateSpawn(
        await interpreter.generateNetwork(1),
        1,
        await interpreter.generateSalt("0x11"),
        await interpreter.generateProgram(
          "0x01",
          await interpreter.generateInstructions([
            await interpreter.generateInstructionByTransfer(
              await interpreter.generateTransferByAccount(await interpreter.generateAccount(owner.address), [
                await interpreter.generateAsset(
                  await interpreter.generateAssetId("1"),
                  await interpreter.generateBalanceByAbsolute(await interpreter.generateAbsolute("100"))
                ),
              ])
            ),
          ])
        ),
        [
          await interpreter.generateAsset(
            await interpreter.generateAssetId("1"),
            await interpreter.generateBalanceByAbsolute(await interpreter.generateAbsolute(200))
          ),
        ]
      )
    );
  });

  it("test generating instruction by spawn", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createInstruction(
            xcvm.createSpawn(
              xcvm.createNetwork(1),
              xcvm.createSalt("0x11"),
              1,
              xcvm.createProgram(
                ethers.utils.hexlify("0x01"),
                xcvm.createInstructions([
                  xcvm.createInstruction(
                    xcvm.createTransfer(xcvm.createAccount(owner.address), [
                      xcvm.createAsset(xcvm.createAssetId(1), xcvm.createBalance(xcvm.createAbsolute("100"))),
                    ])
                  ),
                ])
              ),
              [xcvm.createAsset(xcvm.createAssetId(1), xcvm.createBalance(xcvm.createAbsolute(200)))]
            )
          )
        )
      )
    ).to.be.equal(
      await interpreter.generateInstructionBySpawn(
        await interpreter.generateSpawn(
          await interpreter.generateNetwork(1),
          1,
          await interpreter.generateSalt("0x11"),
          await interpreter.generateProgram(
            "0x01",
            await interpreter.generateInstructions([
              await interpreter.generateInstructionByTransfer(
                await interpreter.generateTransferByAccount(await interpreter.generateAccount(owner.address), [
                  await interpreter.generateAsset(
                    await interpreter.generateAssetId("1"),
                    await interpreter.generateBalanceByAbsolute(await interpreter.generateAbsolute("100"))
                  ),
                ])
              ),
            ])
          ),
          [
            await interpreter.generateAsset(
              await interpreter.generateAssetId("1"),
              await interpreter.generateBalanceByAbsolute(await interpreter.generateAbsolute(200))
            ),
          ]
        )
      )
    );
  });

  it("test generating call", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);

    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createCall(
            ethers.utils.arrayify("0x11"),
            xcvm.createBindings([
              xcvm.createBinding(
                1,
                xcvm.createBindingValue(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200")))
              ),
              xcvm.createBinding(2, xcvm.createBindingValue(xcvm.createAssetId(1))),
            ])
          )
        )
      )
    ).to.be.equal(
      await interpreter.generateCall(
        "0x11",
        await interpreter.generateBindings([
          await interpreter.generateBinding(
            1,
            await interpreter.generateBindingValueByAssetAmount(
              await interpreter.generateAssetAmount(
                await interpreter.generateAssetId("1"),
                await interpreter.generateRatio("100", "200")
              )
            )
          ),
          await interpreter.generateBinding(
            2,
            await interpreter.generateBindingValueByAssetId(await interpreter.generateAssetId("1"))
          ),
        ])
      )
    );
  });

  it("test generating instruction by call", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);

    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createInstruction(
            xcvm.createCall(
              ethers.utils.arrayify("0x11"),
              xcvm.createBindings([
                xcvm.createBinding(
                  1,
                  xcvm.createBindingValue(xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200")))
                ),
                xcvm.createBinding(2, xcvm.createBindingValue(xcvm.createAssetId(1))),
              ])
            )
          )
        )
      )
    ).to.be.equal(
      await interpreter.generateInstructionByCall(
        await interpreter.generateCall(
          "0x11",
          await interpreter.generateBindings([
            await interpreter.generateBinding(
              1,
              await interpreter.generateBindingValueByAssetAmount(
                await interpreter.generateAssetAmount(
                  await interpreter.generateAssetId("1"),
                  await interpreter.generateRatio("100", "200")
                )
              )
            ),
            await interpreter.generateBinding(
              2,
              await interpreter.generateBindingValueByAssetId(await interpreter.generateAssetId("1"))
            ),
          ])
        )
      )
    );
  });

  it("test generating instructions", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);

    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createInstructions([
            xcvm.createInstruction(
              xcvm.createCall(
                ethers.utils.arrayify("0x11"),
                xcvm.createBindings([
                  xcvm.createBinding(
                    1,
                    xcvm.createBindingValue(
                      xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200"))
                    )
                  ),
                  xcvm.createBinding(2, xcvm.createBindingValue(xcvm.createAssetId(1))),
                ])
              )
            ),
          ])
        )
      )
    ).to.be.equal(
      await interpreter.generateInstructions([
        await interpreter.generateInstructionByCall(
          await interpreter.generateCall(
            "0x11",
            await interpreter.generateBindings([
              await interpreter.generateBinding(
                1,
                await interpreter.generateBindingValueByAssetAmount(
                  await interpreter.generateAssetAmount(
                    await interpreter.generateAssetId("1"),
                    await interpreter.generateRatio("100", "200")
                  )
                )
              ),
              await interpreter.generateBinding(
                2,
                await interpreter.generateBindingValueByAssetId(await interpreter.generateAssetId("1"))
              ),
            ])
          )
        ),
      ])
    );
  });

  it("test generating program", async function () {
    let xcvm = new XCVM();
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);

    expect(
      ethers.utils.hexlify(
        xcvm.encodeMessage(
          xcvm.createProgram(
            ethers.utils.hexlify("0x11"),
            xcvm.createInstructions([
              xcvm.createInstruction(
                xcvm.createCall(
                  ethers.utils.arrayify("0x11"),
                  xcvm.createBindings([
                    xcvm.createBinding(
                      1,
                      xcvm.createBindingValue(
                        xcvm.createAssetAmount(xcvm.createAssetId(1), xcvm.createRatio("100", "200"))
                      )
                    ),
                    xcvm.createBinding(2, xcvm.createBindingValue(xcvm.createAssetId(1))),
                  ])
                )
              ),
            ])
          )
        )
      )
    ).to.be.equal(
      await interpreter.generateProgram(
        "0x11",
        await interpreter.generateInstructions([
          await interpreter.generateInstructionByCall(
            await interpreter.generateCall(
              "0x11",
              await interpreter.generateBindings([
                await interpreter.generateBinding(
                  1,
                  await interpreter.generateBindingValueByAssetAmount(
                    await interpreter.generateAssetAmount(
                      await interpreter.generateAssetId("1"),
                      await interpreter.generateRatio("100", "200")
                    )
                  )
                ),
                await interpreter.generateBinding(
                  2,
                  await interpreter.generateBindingValueByAssetId(await interpreter.generateAssetId("1"))
                ),
              ])
            )
          ),
        ])
      )
    );
  });

  it("test addOwner and removeOwner by Call instruction and self later binding", async function () {
    const abiCoder = ethers.utils.defaultAbiCoder;

    let functionSignature = Interpreter.interface.getSighash("addOwners(address[])");
    // placeholder 1
    let payload = ethers.utils.concat([
      ethers.utils.arrayify("0x01"),
      ethers.utils.arrayify(functionSignature),
      abiCoder.encode(["address[]"], [[user1.address]]),
    ]);
    let xcvm = new XCVM();
    let programMessage = xcvm.createProgram(
      "0x01",
      xcvm.createInstructions([
        xcvm.createInstruction(
          xcvm.createCall(
            payload,
            xcvm.createBindings([xcvm.createBinding(0, xcvm.createBindingValue(xcvm.createSelf()))])
          )
        ),
      ])
    );

    let encodedProgram = xcvm.encodeMessage(programMessage);
    let interpreter = await ethers.getContractAt("Interpreter", interpreterAddress);
    expect((await interpreter.owners(user1.address)).toString()).to.be.equal("false");
    await router.runProgram({ networkId: 1, account: owner.address }, encodedProgram, [], []);
    expect((await interpreter.owners(user1.address)).toString()).to.be.equal("true");

    functionSignature = Interpreter.interface.getSighash("removeOwners(address[])");
    // placeholder 1
    payload = ethers.utils.concat([
      ethers.utils.arrayify("0x01"),
      ethers.utils.arrayify(functionSignature),
      abiCoder.encode(["address[]"], [[user1.address]]),
    ]);
    programMessage = xcvm.createProgram(
      "0x01",
      xcvm.createInstructions([
        xcvm.createInstruction(
          xcvm.createCall(
            payload,
            xcvm.createBindings([xcvm.createBinding(0, xcvm.createBindingValue(xcvm.createSelf()))])
          )
        ),
      ])
    );

    encodedProgram = xcvm.encodeMessage(programMessage);
    await router.runProgram({ networkId: 1, account: owner.address }, encodedProgram, [], []);
    expect((await interpreter.owners(user1.address)).toString()).to.be.equal("false");
  });
});