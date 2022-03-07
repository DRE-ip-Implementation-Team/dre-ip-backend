// Create an admin account.
db.admins.insertOne({
  username: "admin",
  // Password is `admin`
  password_hash: "$argon2i$v=19$m=4096,t=3,p=1$WDluTWJ4ZHUyOXRwaUJtcg$PRV0VWV3M4uNMiYZ/85QqB8rkVLHuWoyCq+CkRb9Opw"
});

// Create an election.
let month_milliseconds = 1000 * 60 * 60 * 24 * 30;
let now = ISODate();
let month_after_now = new Date(now.getTime() + month_milliseconds);
let month_after_that = new Date(month_after_now.getTime() + month_milliseconds);

db.elections.insertOne({
  "_id": ObjectId("622650f453036aff34eb72a7"),
  "name": "Test Election",
  "finalised": true,
  "start_time": now,
  "end_time": month_after_now,
  "electorates": {
    "Courses": {"name": "Courses", "groups": ["Physics", "CompSci", "Maths"], "is_mutex": true},
    "Societies": {"name": "Societies", "groups": ["Quidditch", "Moongolf", "CompSoc"], "is_mutex": false}
  },
  "questions": {
    "622650f453036aff34eb72a4": {
      "id": ObjectId("622650f453036aff34eb72a4"),
      "description": "Should CompSoc host a talk about Quantum Cryptography?",
      "constraints": {"Courses": ["CompSci"], "Societies": ["CompSoc"]},
      "candidates": ["Yes", "No"]
    },
    "622650f453036aff34eb72a2": {
      "id": ObjectId("622650f453036aff34eb72a2"),
      "description": "Who should be captain of the Quidditch team?",
      "constraints": {"Societies": ["Quidditch"]},
      "candidates": ["Chris Riches", "Parry Hotter"]
    },
    "622650f453036aff34eb72a3": {
      "id": ObjectId("622650f453036aff34eb72a3"),
      "description": "Who should be president of Warwick Extreme Moongolf?",
      "constraints": {"Societies": ["Moongolf"]},
      "candidates": ["John Smith", "Jane Doe"]
    }
  },
  "crypto": {
    "g1": "A2sX0fLhLEJH-Lzm5WOkQPJ3A32BLeszoPShOUXYmMKW",
    "g2": "A-syy9dgQJ4rOGKtzS6RK5ByfNbSlpNNZrcLL5hlACBY",
    "private_key": "TtRlBXsDVyo5b70DTK_5nq4rpzY6KWk79U_X_ZEvuOQ",
    "public_key": "AvD2uFnvrMBSVJxAhgzY4NUOWf2xI07sbPe6qkC6MBYK"
  }
});

// Create a non-finalised election.
db.elections.insertOne({
  "_id": ObjectId("622651a81692ca8e92a9879a"),
  "name": "Unfinalised Election",
  "finalised": false,
  "start_time": month_after_now,
  "end_time": month_after_that,
  "electorates": {
    "Societies": {
      "name": "Societies",
      "groups": ["Quidditch", "Moongolf", "CompSoc"],
      "is_mutex": false
    }
  },
  "questions": {
    "622651a81692ca8e92a98798": {
      "id": ObjectId("622651a81692ca8e92a98798"),
      "description": "Who should be president of Warwick Extreme Moongolf?",
      "constraints": {"Societies": ["Moongolf"]},
      "candidates": ["John Smith", "Jane Doe"]
    },
    "622651a81692ca8e92a98797": {
      "id": ObjectId("622651a81692ca8e92a98797"),
      "description": "Who should be captain of the Quidditch team?",
      "constraints": {"Societies": ["Quidditch"]},
      "candidates": ["Chris Riches", "Parry Hotter"]
    }
  },
  "crypto": {
    "g1": "A2sX0fLhLEJH-Lzm5WOkQPJ3A32BLeszoPShOUXYmMKW",
    "g2": "Alx7R-Ae-8iiXsQWeIZ98TqjVVwS46KQNivQqJ1N9ELL",
    "private_key": "f3u3QCDe16kHwLygC718KOdhw9KCdHnSvGcUKSxmBD4",
    "public_key": "AktRivXXYqlaIYq8-oYmA-ccvDupIz_aTXEgj2Y4UmAV"
  }
});

// Create some existing ballots.
db.ballots.insertMany([
  {"_id":ObjectId("622650f453036aff34eb72a9"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"creation_time":{"$date":{"$numberLong":"1646678260136"}},"votes":{"Parry Hotter":{"R":"AgpauoppZMIRBAS5Cnfai2JTe7YrptN2nJsZIrRam2YX","Z":"A8wGn_D2ukybshPRiJuV0rZgQMpDXFn38d0HN0gZBiY5","pwf":{"c1":"VXZ-v3wBZzSOtJvcTpgEdXuTYK5DcNBe_Fx8m9NHnaM","c2":"lFztjV7wb9DHTOkUq3puBRpiF2Cw7iiubBTtIMblEY8","r1":"epE-IYEj0ZQQ8BSRNk6cPwWB2u9ZN8d5kgdwuFsNRZ4","r2":"nOI9Qb_F3rWMi0YAH7m4vX3u-_XO-fZdkEY4jWXrJZM"}},"Chris Riches":{"R":"A1azlRb4sjKmpKXakYVp6SoAJmpxm2rGoeHb9xiFXGqx","Z":"AtXcOmPYA7eBNY8wvGAVNeNQ5p8Kio9fcyTqUusBYWc8","pwf":{"c1":"gLzfc9r0R429kOC_N1AbcozjiLK8jgpnUPmVbc_qLDk","c2":"UwHNfCuymPCoRTlYv6kJSTPL4qvAz5Nics6uTtinn9k","r1":"_391ykxqa1tYs_8T5evbWWNcs8wwY7EPxTV6oFPoLOw","r2":"16PdUGLjd3tQ_kTDL2J4zSR6hMP2yPnaokIzowd58_E"}}},"pwf":{"a":"AlUDTWF4vS-eYef4THljiHlLy7Bpx1Y6UMSWClP5JKYL","b":"A5xK2PF_pyT4hOX5UOdwfmF9PcioNLKIyoUAIjmVg8kG","r":"KoAzy8g_xRBXGO5gkSgJJRfdng-FI__oWEPdoQ1h-EY"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72aa"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"creation_time":{"$date":{"$numberLong":"1646678260173"}},"votes":{"Parry Hotter":{"R":"Al43uHkzxKxZdq8wNu0GHS0pELB3RtqWLf4l4lXYF_zv","Z":"Apu42bIn8-1ozTk5AbPTcRLSwGR2z36jIKgSFacwYemW","pwf":{"c1":"lp6OqachicsYD5a_4x4BqhsjXkQ8ZFTGTEyTLnmetqE","c2":"d_bo_hcHpDA_U65PDkAAXlP46ru89BRrarU_X0GXTlI","r1":"smY7wc2g5LGsISFANfbjVt1AojUaaQWiBBlvCoeIShg","r2":"xzYi0Z6JDOj8xxHb3v9mLsrp_sEWWDz1QZk4fZSBVqw"}},"Chris Riches":{"R":"A6lFLq_IsZbvQkrWJywMTmRsHToS_NSrtDAdjGCpoHIh","Z":"ApV_VsvhOF9JWp7YMu-CiXsA4qLu0ckI1B56zIxKMy0c","pwf":{"c1":"FLcpV7vpwio85tLZ21-9XvkbmSt8TK7rsikFs7Jji_U","c2":"2FcGsdAWbVSbGjvF2sSCy8RueUtdbxFs0QBMkfuMbbg","r1":"zvfdO75ujHvPIm8aY5bpRMnek43xhzB3s6Wgt6oFBaY","r2":"iP1OrjPVqX7vw2jrZ9-8RF2JOm6mBIfFz5ZwXE1EMjg"}}},"pwf":{"a":"AvdFsNaIoby_CIzADYKt6E1lVcjwQ6ymZHaD9sTC7tl1","b":"AltRGi35jLhxR6qU5ThL5Wjek6KfA92mewgQrG4ii7Gi","r":"x57UWduW2n2ZnznNne3YCwqs1EKdoo6T7QOnPOYht58"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72ab"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"creation_time":{"$date":{"$numberLong":"1646678260210"}},"votes":{"Chris Riches":{"R":"AwqwB4wKCY79oXHMmskDEyPbCBEvsYYh5Mc_4RotT9ok","Z":"Alndw13PiwPbvPRjx3R1VrFE20U2_fdOsc7lC7kicesN","pwf":{"c1":"8bT3qfwKwp9U0n3wwladv3SHRfqTl5XJjSN9DLDuafg","c2":"Yp-1OeU-ydB87nfFLdz8juAMQM6n0nP3C68ODBOMauw","r1":"-05sj2yMzEnDYtqaw1o4Ym015CEoCIPpCUDo_Zw0BMs","r2":"k6SGynO1bXNKG4iLruADttWkf1wDfzvKoOpJols_nbY"}},"Parry Hotter":{"R":"AmZy18LIxzYY4iAkNPvveFUP3Vu4l3GpyUIRID-bBWOy","Z":"A_kVwSTzfL1sPRX84TAQCtCG5yB_Z_Ckkkj6jwZCJU4y","pwf":{"c1":"d83YFSQ9JbOwOAX39JvkgOiaxmdrKaB3g3bHb_yTcXQ","c2":"Tuqx6C1DQgkjjLv5hiwYxGWUOGRaNzxfx5TIwSEiGQE","r1":"c2m3RTuGxo7Usj1RNuCNzz6BTSWhjp_v3E3Rnsne6Lk","r2":"XEMRIlqqlTcFpngdguURCimm5aSdrKIeCElJfN30pZg"}}},"pwf":{"a":"AmtTm4vPUk6WJCnIj-XwCE8nlHNoHuKEp-hDpT2jTI-h","b":"A-ZBN5sHbBTFjKyh8g76Pf-q5SOdCoqnfzdcR8JCThwJ","r":"gjrjrX6OqFH8bD5TSyRyrW4_99S1JY4GKoPbSBRcqAg"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72ac"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"creation_time":{"$date":{"$numberLong":"1646678260247"}},"votes":{"Chris Riches":{"R":"ApacdpNR_4cWZSk_Sk7CGAPgkGYzwQkk6-sSvLsNsg35","Z":"Ap_psPpRgpMYogRlkEZzpI3bpxlrSjKpXC_5l0pyP1VW","pwf":{"c1":"pEUhHsvl_Wm9EDYBdRfnr821IbzlBYMfKgSz_Vf5dwg","c2":"pwergzjZSv9iI-RI08gvsTw0eOnNul6-vx4uKhhFSRI","r1":"SVa1e26HHdTvCOMuy33FeWQSESKZG6GH0Ebe4Olah4w","r2":"G6N-Sp7A_pXVzwkbl8nmJUuSA8-iakw8rSusk1ZuACo"}},"Parry Hotter":{"R":"A7Xvk4bLAcYPj0a5UprA-bXWtL8aDj2fbH8_wv5JCAEy","Z":"A-cbEUE0iVoZBbCJ9uoy3lmElwITGsZm8QCvSQ7qyXBE","pwf":{"c1":"R-48VHDUWmY_p-y7WeNIWMieREHzT5mY254pSlzODe8","c2":"reifDDoUGGQWsRY9J3NN6LJwNTXV6tc9rBIXzVokgyk","r1":"RsY0VU2fqnZVS6dtr4Li3tvxpQOjXMDF5RRJHDQIMlU","r2":"Ft-0cgUTk2Y0tbzpKw6cOeEqD566AwLoEwo_Uwucdv4"}}},"pwf":{"a":"AhCUSqVMEHZB2uYIgDsm7UQB7qLP775gB9OZvKbbPfVB","b":"A7VRfsHZay6S6hFabpGqJb2KelU6-0mku_8J3efw1etk","r":"S3FgYqsHaf57Y2i9verq6l7b5mU21GyUCVz5toXHuug"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72ad"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"creation_time":{"$date":{"$numberLong":"1646678260283"}},"votes":{"Chris Riches":{"R":"A7LIi_1gDRyqU7kj4r3pw2fWoqDzohHZ4SJadA36N607","Z":"A4e-h819K8cXvTQJsrj-FwAsfblekHZ9yTzhQXhH1yqd","pwf":{"c1":"L-33XS88DCw66mxj7vHWeAeZWB0g5pucStQRfm17R9g","c2":"dRs7j3MsaOO2gk7U6AHJoxKfs1yHBmLHCqS1WGRgYnI","r1":"K6np9xw6LVnEpNVl19kq27oo8DQrr_Tg2oQmR2ZT5Zk","r2":"rEkixaRNjb4TL4uK9heGBrpe8xWeQXvvoThQE-vSls8"}},"Parry Hotter":{"R":"AmKjMGa29DaZtnVU7SNbWL9xL3voyI7LBcLXgeuHrL6K","Z":"AlNSHNBPU6f8BAfu5H0nx8aeUhRwplkFJkykUKILTAnc","pwf":{"c1":"EReahjbBTHrDEelz1g4HlwQrlyu_TkXwNJERRuaAFgM","c2":"ephBffktlo0JuprsbNrQwHSEWTHIgc2dl9NfhyzC9a0","r1":"vKnHa-9oEtA9rxSb4jQLq3LjYoxRAL8Cm4bPGSEiYtM","r2":"PeWGeSTN3ZeC0p2L-uKv85SKINKLVuC8EMoijj67YQ8"}}},"pwf":{"a":"A0Xhydyb7b8M1jl2ck89THkvhlYamZzmPQV1cCxSTLJ0","b":"A1aPfmQny3LTADy949lswamFIMFIgxpEqGNwMb907dQp","r":"1RMOSps_tqdU_ACEMnXx7SfTTe30CV0iur_CGaiE8R0"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72b1"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"creation_time":{"$date":{"$numberLong":"1646678260430"}},"votes":{"Parry Hotter":{"r":"qD8VP9G4EWTQzhPmNbMconIoD4sft6T9vahi5faxJ-k","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"AjDtgbQYtAyE_vT5zRLciroZtCWSsz59vRDgfp13bmEL","Z":"A5aRugOlAdFGKfmfQx2lRiX9f5Nnk9joI3R92BJoVgCa","pwf":{"c1":"1nO51CEfUGvKsDMbDcHOTWpST6j_6kuKEcwUIWKd-tc","c2":"x26qpX5C06bG-y5U1_q-N39ESCZpNqwF1WNU3rHbGn8","r1":"SM-uuzqNG4P-G1XYXhAojj_66kzbwMA793Uc2Iwryi8","r2":"Lqor5DPAuUkBm6a3kpy15FQ0VBDCRCTUHraJ24CpXxA"}},"Chris Riches":{"r":"4E8dhxVhSxh25xphVLAHxyhm8sr62mUHjQa3EOzQ5Lg","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"AqTochKrzijkDQqPJ9H4C2jkmJ-S5IlnTLcDx-OxEQDj","Z":"A5uNOjH3ZPst7MaPDwCnIWm3kgv_u1tls1wKHCIKauRl","pwf":{"c1":"b5AoW9YPOZ-_0Go1lmHB47vhwn_4cj3iT7E52HOuGAA","c2":"cOizEtuee-M6WvjlqNiNY0mOzBaED7HSs4nC--iJeM8","r1":"_SpvIc51jOoiNv14HuhCbfoAePLZqG4utY0baTj4HDY","r2":"vZAclxo_vxScNlqwcvKrsIsANXWYpBOyzpdEVol9E38"}}},"pwf":{"a":"A5K5CHC7eDVYnfGg103ZKnN-VV2vbeKsffSdDMIA-mBX","b":"ApDWRpqJfgS3jwT0LDgJoE0f0fZQuTFq2l_nzHRfa_Du","r":"OZIrh6Aa3vot0uwKdZDr6Q9zJV5Xu9kZoM8L49KouG8"},"state":"Audited"},
  {"_id":ObjectId("622650f453036aff34eb72b2"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"creation_time":{"$date":{"$numberLong":"1646678260467"}},"votes":{"Chris Riches":{"r":"qF4RXqtGGlNFF6WDfleDsmsYvpILVIjH_Eix2YhEWs0","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"AzIKdSc5PNOW6Nh8uqf6MIbU8lE5WU7J1UXdk2r-wnIW","Z":"AtmtvidrWr6uUmAq-zHaT6qcKt04YhYycQrIA_MWNqDs","pwf":{"c1":"37OGJGDs2KAB9GJ6_Uh9mtmg9aj73bMLnGqHjspcrkU","c2":"NLU9AXkqiB-KdLFdHwCGMCukFMeZ5xORcXuHwq69Rh0","r1":"nlCyZPbV9J3RhaFyLxLzJL3aGaIZEkosyk18pXWLVxQ","r2":"f4zS08nOdxnjP7_oo6nbrGDhFoo1vEoRw9evQ62Gfrs"}},"Parry Hotter":{"r":"vsMoK-yDRKC2pyNM-lui2GGMmEgV9XSgo6AX9JC36-4","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"A8cvxexk7Q-2uiUg-V6OR1KYOaqWpoJd-Qdl5oRUHbA9","Z":"AjmJGSVzo5Djdl2l755teTTpcWVI0tZQrjufLOthboQE","pwf":{"c1":"PNficc3G1PcUez3tPiNH0sUF-Rka3BmntGokzMX-AeM","c2":"TKV2Uczd6CGsP4l9O5Oc_ZjoDSFaLjuZ3jK6xwreZSU","r1":"5x-EuuGYX9A-sDVx3vrelXgt2ec0iQM2oO4ST7O6FtY","r2":"bN9QKq00wg1z2TVxiSKUDwwWia6D1JlBKti_fjUVaZU"}}},"pwf":{"a":"A_B7HcApW9e1uPwNbgJ1UfIN12a8S40C_M9BRgLMgd0_","b":"AnP0luf10aKns-l6Fgp2cWcCCPUS1Et06P-ErFMycqEX","r":"QwciISx6isnNhhBS-6QXWz-Je4K8OgPxjQ6YPhXf6r8"},"state":"Audited"},
  {"_id":ObjectId("622650f453036aff34eb72ae"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"creation_time":{"$date":{"$numberLong":"1646678260320"}},"votes":{"Jane Doe":{"R":"AsynR7hteGy-x0HC2TAPHuTConnaEIn2iorcz7sR3Nug","Z":"AinbKG7zlHCCpTpOXTxeKI-Kx7H3g54NArcsANn0T40d","pwf":{"c1":"jS2uCsm33qNR8KOORp3yHkSqPqAmzl5wmU6fXEOD6nY","c2":"hrlWRB-jP8wDWCBboudzF39P-7rszxITsD0xFRZtP2M","r1":"4-Qtad-kCr5exrh5ytuXHrxe-jboWCb7IwNdKuz0a5o","r2":"xLBYYpePoqBpuT0yL5FlSY_VYzJ6sYgjG0RzARswqYI"}},"John Smith":{"R":"A7JVWQsmumIJIrZDQ0--Yah_HWpLMIdci35tTVH3y475","Z":"A72r30QWhMF3STbDA-MFacyQGT3fUKnrLz5qqnvEtQpg","pwf":{"c1":"aie4hHWKWX1CuAzdFXJsiiJi0v0iiVG8GCgURqQqJtw","c2":"5DXp0LsaGbsgDCor3Ennx7BCVWG1uAe2WqX4IrmNt9o","r1":"8H6iQ18F34FKMon_sLJkPAf0EeWhKZE-2Niv4FmF7t8","r2":"MBKpcYGJsXLYADR6aOL5PN90QPlg2D1aoTGdmC8u8EE"}}},"pwf":{"a":"A_o9ZtVhS1K6t_ES9lCe6NOFCmqZPK14n5rkUnXB7FMk","b":"A6mejDTEBorbR0dDOLmtZftxEbqryGA4gmluxRftzZIl","r":"NFGGgat1Xw7Jk1UxCQVL7-pjAPOe04DxiWP6E14T8-g"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72af"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"creation_time":{"$date":{"$numberLong":"1646678260357"}},"votes":{"John Smith":{"R":"AoLnbAE_xuo8002ss6KeU7EGSpIr4v2thxxRWH0a7Q6N","Z":"Atym3A2RHyx5Llo7gilp7z8fcJgvwXRcV8xbdw19fn-a","pwf":{"c1":"8qTrmSVtNo5Ln-Ga5RC5d-PZnfiaTVTxCnHMF6ExAQo","c2":"yCRUkZM90VUz0QM4UixhIglOC-BhaKXSO7dmvVND5UQ","r1":"6s40QUOpz8XqBeXweGCBH6Pq0Z6kOMcO20Z-3Rg8Rhs","r2":"nBfiEBHGUIH4iZfFZDqAcjnFan4E2QMypRkjdxdglVc"}},"Jane Doe":{"R":"AmhMa1goS_4dmOwVIRM4YgMNORzbTUeq44TxglVhvGYv","Z":"As2aeGWf679-n9MaG3laqCe2XO2qGenz_qFzvw9qBgs5","pwf":{"c1":"O-J9gcnA3XOF9aqOYt1kDfB3UCMlM7Dz_KuW3FUrdbI","c2":"myMCQ6icFEgQHcUVK2lk_1wo5gaNjE80xeq-ZDIbck8","r1":"iI0hGHOavZqqrtq9EF91Lq1a3zsqBzoL2yv4BGX4JZY","r2":"cIGTDOLlA3yWAh5SsD6vjPWdCMcDEjyQ8e6Sb0v0oNQ"}}},"pwf":{"a":"A2YIeJsqkQlDjzprjxpSKu3-My1cZ5fEgN9GlAsFRD6P","b":"A9kqkCBdi4MJB78L5oa8uJXZ-4NTB5qbbeRaS-18HnpZ","r":"57CseDzFGKgdYrb0z3DKZcJD0HuFb5r2oYvyiMEL3U8"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72b0"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"creation_time":{"$date":{"$numberLong":"1646678260394"}},"votes":{"Jane Doe":{"R":"Agk0dHxdFLlK-7aMr-NEVilpvO1C8jsJ_0_rvj6qUoUX","Z":"AwkDfdCYTcMP1uU3khclukOuCPdoCN1ThmYsqJlQ2O6x","pwf":{"c1":"EoG2n1utgPuk-VbS-1uVXoQlpM4njRRsS0WefonOoJ4","c2":"JB5AAK2v1GjJBXoKWSt6GJaDVd-vSouOlfQTgNAOFlE","r1":"lzgLz8LLUe7J7naCSkMPscTyTqmhjkYsjq9Ej-5mpC4","r2":"DN-wWi8fP35jskQ8oixEv25AAxGI0P4v0Qx7gF-Vjeo"}},"John Smith":{"R":"A0dTAd46L1ReQ5WCRM71rOov3W4n6C9aTuL9q-r8emd_","Z":"AhdxwBrVALjmIZcponz1Zlx11_lXIYkVdz67rZ45rIMN","pwf":{"c1":"o0tDQAugtvhLxbK3w7r4HKVM7ye9OMs_tIpVLXcybC0","c2":"wcRuAg3FCSX12r9v6rLuLmwbSIrsGs7EOiRGJtIJ5zk","r1":"OHZAdBdPrLL6QvSoQ0oy2UQPtK4AvPvwGQIklrqXhWw","r2":"z1WFRewYXBmdu56Bpd0NIzZJs1DMGOxqfNXfYAZlVRQ"}}},"pwf":{"a":"A6UFiSlWP9SqnTdxxilwDGg9hSDpzXhAjULGkQEtBg-q","b":"A-63eujIsf_spDDE0MPkPej0E1-rxnRewCclSVhXBcHx","r":"TG55hHpfU2-sp1xVWqDzuTaSrjrgQo5ULDUhFuq3ke0"},"state":"Confirmed"},
  {"_id":ObjectId("622650f453036aff34eb72b3"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"creation_time":{"$date":{"$numberLong":"1646678260504"}},"votes":{"John Smith":{"r":"rii_a8Xseuov0god84xB3jrMhYaGzZIGp5IhItKYFxU","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"A36s-7KXJr2Wbm3T50rYgVZW8E99Bi1_UH6uDjBJX_kO","Z":"A-zCiq5k5ERK_ylWqb4Hbp7NblqaIqDvbCrId0UMTAmf","pwf":{"c1":"WSTKks5QCVRDF4WC3yanSXixBfjieZTQ8irDT2rDQl0","c2":"TgNXAiy3lDFHbajiM6clVF_5OoSeaim746Rl29ulfOc","r1":"jdRr2lbBKcGvAunz0VrFyJGQIRTybF5DJcOITQ2EuS0","r2":"2pmtvLaZwp8rZVE4Ypfz-pk3U3V1T3XcXzM-KutYjcI"}},"Jane Doe":{"r":"N43mfSen6akIABQXrfwe14-3j2IeYPw2Gc5QfbB0ltE","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"AhkU5l2XWGX0EWFUqyN5tKOTzvimXimnEt0aGtL6jaOR","Z":"All38xIOmNLvOz9zxMiZtenzF2KbWQp4QSrrhMNANi50","pwf":{"c1":"M13E_fRg1zrS1eNSOZF0DOwwFfl49-Tn_KRk0pP46uw","c2":"PRfLeiJISfQsoU-BEAGw01dX3hS_9fA3lUm64GzgeF4","r1":"bY4sfU8etjkPpuQ_nIEtF_DDxBLQpARTN7RukdkifQk","r2":"K3Uz4QjeRf1X3UUybVBRvNnoVyJM2oK2r6seeHy1lNU"}}},"pwf":{"a":"AxmJGWSyuGFXmuSNqG0Xwhd266j5StcNAsMoLEBxvMTn","b":"A28bqGq_uWLYgd99-r_KHB-TSd9yV9IWUX2ULt7ql7GQ","r":"_x6Lxg5jKsIyLI2Y7PLzixfMERcVbYu-CV1qeKmfc-U"},"state":"Audited"},
  {"_id":ObjectId("622650f453036aff34eb72b4"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"creation_time":{"$date":{"$numberLong":"1646678260541"}},"votes":{"John Smith":{"r":"k4iKa0MMWdfb92Kof8Nr7Iy2A7Xl6Mh9Q14ll7KT3mQ","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"A6Sy5scWqBoaTysFmmcJ5B52tWUqqELu8O5u2fbPx9CM","Z":"Asw0h0nhPM1waW-I2FAi0WmqxQPyxFteKHViIB0YWLRv","pwf":{"c1":"NKtsIv6fQ9lbSzqgHQ-TGYjiIPcB-s0fCy6kLyXOhGA","c2":"9qV_Zf4axrELIUNAPYrpBmOVldSawiMsqtBLh5_EqXU","r1":"NOLUfay6eB_dTOFTu7L45OOVqyoTAvuQ8HKyvFlLQ4c","r2":"eymQb1369D5-VaAiSFUGexyYmYKPr7iQUvlp9-U-18o"}},"Jane Doe":{"r":"rDU9vTUFxV_MpixddCc4lZz9P8dWRG6BMXorhGBKhI8","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"Aqcz9LnDvyiHZnBTlr0dVs_DF_nNSq2MqiMz8ramE-8J","Z":"A_O45CJ7wKCk5JVdKBeoqIevXk73QKivAJ_anyBIKhge","pwf":{"c1":"zPJU15zasaKgKvuEeIKdJWDaKiw5T0DMfXUi5n91sUw","c2":"p1J0CFhJrWSPpj5qSDmK9jutqoLrmp_qTw1MrwBYaS8","r1":"1s50TEcPiS8zccn4_Hz174z1NeeJOwBqi78JmBAvAes","r2":"GClMkU0mFmyC1CktL7xO3dRLEUDyr-4Fz-SA-XgU9wg"}}},"pwf":{"a":"A3sqAFZ19uyzUXnOGg6n2nC_feD3X2slc5MSKlbXY1Q4","b":"A80tw4_PaKTcXfD7qIDV0PdkoBSK78pvHXpu7gXflymk","r":"BPccKR_dXsMOvfH8C9pmEaeqaPOVeygBkKCDqHoBw9Y"},"state":"Audited"},
  {"_id":ObjectId("622650f453036aff34eb72b5"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"creation_time":{"$date":{"$numberLong":"1646678260578"}},"votes":{"John Smith":{"r":"4P7gRaqzWqE04Rbruq2IB-IksxguKZRQVToX1tsY92c","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"AoMCCuMQWDql2-McTs4pu2me9-JAd23S6dK9yFPTOCAz","Z":"A3tMZ-eWsVNAME5x4hJjEHG1wqTn1Hg8lVNWPsCzIert","pwf":{"c1":"A_ieSHx3RqIoNJ3oilARp6yiEDRFNF8ONt6Wp0NqHx8","c2":"a_OJrZPPPqIBH48DTr7vE-0goqhd3m1rABJQO87LVrY","r1":"46QxMXMYSel-VVFW1uevTSsBCsmxTVLaLpZtEglDxoU","r2":"uWY1CpJeLGKL7PwxsILESzcSnDbOpVdTBfNeoQf65Gs"}},"Jane Doe":{"r":"7SxKSjI_8T11eCtummwSGZ7se6Rij98FDTsw7TX855w","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"Az3qT8FblEhjgXNCXYM7wd2g-nNvg9Z2etM2j7jWOZYQ","Z":"Au_pPhqwHGt22r9Mib6CP0P6s61rh8iu1rEmz4hKNZBC","pwf":{"c1":"tx0CV3BaQ-uT177FvGN8mLnHfEpZPKY-gixbavRkFx4","c2":"VYM2e8stt46CKynNi0vqd87QJZZPqCnsFQTmKuGaKfg","r1":"4tEunTFwHWFU4kOdgNXPwQJTBoJry1IoMrI9I8eWn2g","r2":"loHVr4NywF9XlqHpGJsJwR25lQB7JgbfjFY-2aZVviY"}}},"pwf":{"a":"AwrckYzbI9fGJ_CbqTAn3hBYBbymGCePB9RVp1JyGfux","b":"A1reyBuh5nAaq1fHFn7ZLrE9s1sPE02jyXonwROGbeMN","r":"_rTPK8kmPeZhic6llUPhREZjNF7cEXpYbnqkLMUKdOc"},"state":"Audited"},
  {"_id":ObjectId("622650f453036aff34eb72b6"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"creation_time":{"$date":{"$numberLong":"1646678260615"}},"votes":{"John Smith":{"r":"C0HgZoo7Hc5DB5GEwF4fmV2aWXJHzy1reNTz6JIQ9V0","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"Atuon4jENj2zOSbILJJqRdv-GDmw7GplbFTxG5t84Lgm","Z":"A742fiDnr4xLznn6EAXh66MNpyv6DY8a5keUHBPnABxK","pwf":{"c1":"jbENv2PuppLaTb-zVK4y0qtk9x2OAIf-qOzuJQV9CBY","c2":"fFjiPo9I-N2Ztu3FSOujTs1GhkaDDCD5V2DBcQHhimg","r1":"qCRaSCvJzQpxMYAa1Fj4POJLgwuPglF_2v4a1xG-WSA","r2":"PZsLxuSbKqrq0Ak-X9uPQx9VDlQK9zbC7H6ztuDNGQQ"}},"Jane Doe":{"r":"17_KZZSJ7Y5zAgFElFewM2dqOyHWSFHzSprHPpaGGAo","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"At9F8rH-icNMIqHQL4jsq2pgS203JjGfuP0ShuVJ9Mqg","Z":"A7hzQxOzu1XID7TbK85QJT-u2pUvCDWbjuvig54H518m","pwf":{"c1":"wk05jcWAjd2oaeTsim05gYxWPaNr2l2ESPcWOnWQqBQ","c2":"87k2cFCjt0JqRE14evm6hALBzsbWRi-weV2vbW1XsEM","r1":"x3DAykNAsak8tKqUPxsFUNuxUrywl5M1MN1n8uC0oPc","r2":"sbkyr2-uaWCReUW-iFlSr6z4VgwVnnl29xeHtvmsPpQ"}}},"pwf":{"a":"Ao4cRWLfbaNInmEnDNx8KS9uE93pj9JpmkJPFSpxlzob","b":"AsQ2ND4Alt-N2Rxh4chAh_mh0ZqRjHlq8wi3Jf4DoXfj","r":"M1N_smAtPq1b-xBzB2Hq9SJGN8rHzGfpLqGQeQkAAcc"},"state":"Audited"}
]);

// Add the corresponding candidate totals.
db.candidate_totals.insertMany([
  {"_id":ObjectId("622650f453036aff34eb72ba"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"candidate_name":"Chris Riches","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM","r_sum":"BsgLrOYjIS-xjSUodqiU7zUjjwoU8vo3OTiYUfH5D2c"},
  {"_id":ObjectId("622650f453036aff34eb72bb"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a2"),"candidate_name":"Parry Hotter","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAI","r_sum":"MNyIbah8tHwGjMWIsLr6zEm-2xZmlH0e67mAbk3ghqY"},
  {"_id":ObjectId("622650f453036aff34eb72bc"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"candidate_name":"John Smith","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","r_sum":"GfEweCSa0BC4SC7L7LFyYpd3KtOZfxpN-pu1rp3bFk8"},
  {"_id":ObjectId("622650f453036aff34eb72bd"),"election_id":ObjectId("622650f453036aff34eb72a7"),"question_id":ObjectId("622650f453036aff34eb72a3"),"candidate_name":"Jane Doe","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM","r_sum":"FEnv2GdPEjKgNJyP0jg_upScM3LRw9IrzRqrqPRwh4k"}
]);
