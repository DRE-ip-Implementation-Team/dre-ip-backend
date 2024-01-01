// This script inserts test data useful for end-to-end testing.

use dreip;
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

// Create a current election.
db.elections.insertOne({
  "_id": 1,
  "name": "Test Election",
  "state": "Published",
  "start_time": now,
  "end_time": month_after_now,
  "electorates": {
    "Courses": {"name": "Courses", "groups": ["Physics", "CompSci", "Maths"], "is_mutex": true},
    "Societies": {"name": "Societies", "groups": ["Quidditch", "Moongolf", "CompSoc"], "is_mutex": false}
  },
  "questions": {
    "1": {
      "id": 1,
      "description": "Who should be captain of the Quidditch team?",
      "constraints": {"Societies": ["Quidditch"]},
      "candidates": ["Chris Riches", "Parry Hotter"]
    },
    "2": {
      "id": 2,
      "description": "Who should be president of Warwick Extreme Moongolf?",
      "constraints": {"Societies": ["Moongolf"]},
      "candidates": ["John Smith", "Jane Doe"]
    }
  },
  "crypto": {
    "g1": "A2sX0fLhLEJH-Lzm5WOkQPJ3A32BLeszoPShOUXYmMKW",
    "g2": "AwohrHNVIHtBuRPFL_aekHB4R_euUWZnyc1xE6_td3Oi",
    "private_key": "uDwf1pypFCYWVIkgDLU1gu5bU67RFrp8CPIsyt5PV2g",
    "public_key": "AkMX0X9i0boyPTXL8j4-oyXhFSMXQpwlnOTgJFwNFdpX"
  }
});

// Create a draft election.
db.elections.insertOne({
  "_id": 2,
  "name": "Draft Election",
  "state": "Draft",
  "start_time": month_after_now,
  "end_time": month_after_that,
  "electorates": {
    "Courses": {"name": "Courses", "groups": ["Physics", "Maths", "CompSci"], "is_mutex": true},
    "Societies": {"name": "Societies", "groups": ["Moongolf", "Quidditch", "CompSoc"], "is_mutex": false}
  },
  "questions": {
    "1": {
      "id": 1,
      "description": "Who should be captain of the Quidditch team?",
      "constraints": {"Societies": ["Quidditch"]},
      "candidates": ["Chris Riches", "Parry Hotter"]
    },
    "2": {
      "id": 2,
      "description": "Who should be president of Warwick Extreme Moongolf?",
      "constraints": {"Societies": ["Moongolf"]},
      "candidates": ["John Smith", "Jane Doe"]
    },
    "3": {
      "id": 3,
      "description": "Should CompSoc host a talk about Quantum Cryptography?",
      "constraints": {"Courses": ["CompSci"], "Societies": ["CompSoc"]},
      "candidates": ["Yes", "No"]
    }
  },
  "crypto": {
    "g1": "A2sX0fLhLEJH-Lzm5WOkQPJ3A32BLeszoPShOUXYmMKW",
    "g2": "AwohrHNVIHtBuRPFL_aekHB4R_euUWZnyc1xE6_td3Oi",
    "private_key": "b_tTc9ct2DqE4mZoXbyczkkT-piDX_Siw9O6bs6DRCg",
    "public_key": "A6nS6eZbs32TYhNX7XAMeuRlJ3faf8rRc_MwmNPseRjS"
  }
});

// Create some existing ballots.
db.ballots.insertMany([
  {"_id":ObjectId("62671ce89f44880257dd98a1"),"ballot_id":1,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924775670"}},"votes":{"Parry Hotter":{"R":"A9ed85-6nrKpoHY29eFxw9dl5nTS-WJYd6dsnPTsuFWR","Z":"AnDXT3CDw3bV0I1kvjMumwUhZ7QFhcK03HVkr-JfwpK-","pwf":{"c1":"XludC02LuHD5hdjyPzRtRssOhgYwEFwp8BvtK82oGQ0","c2":"ux3DzTYIIDhoa5liEakR7RmUIbqX8s33fpyN80k9YzI","r1":"79yIzIg8lCHrTAnLK3ryY_8VTbj1JqGxQrqchUPGxXY","r2":"6Frqulk6JCwfIRw2JTCGYAt293GgZxID94M-iWBgakA"}},"Chris Riches":{"R":"AjijcYEvwu1PGp5rZoJCko0Swb1zAfsO8T69h00zHpA1","Z":"AmODcPOhiH-s8vNO3jr4c272bX02Y0vIfKcbZkb6DiNI","pwf":{"c1":"nDYaLQXq459KAfbF0qFdmMrej7nSipDsC0s4xP5gCXE","c2":"TtLnI3mdh-DiclkHkSAFxeNWrqUDvRBuyEW96QiwpFg","r1":"at3uS-uGsg6tjgm9vsWUrovURV1Nwsu4zURKBfOiD-o","r2":"Xu0wz431QDVZJ73opDyDxYhjI24Msbt2WFMCvaHnm8Q"}}},"pwf":{"a":"A31m2w4yAjOSo7Q83abVJd7-gdGRN1IZqriuj42wG5yY","b":"AkXWnzIcYgRcwvAnuAJmqUI5jJxVkCOkKG1eCzY6EuPJ","r":"bU4HmXepoZoktSEXtt33wBXPK5hZbzRKELMBLnVxRFY"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a2"),"ballot_id":2,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924775710"}},"votes":{"Chris Riches":{"R":"A8llaVFYx-SIY7VJw_dnO7Rr2mopKwyHVnNIFKHxQfz0","Z":"AvgaE6t94-3zNwSIDH3pHAPF3lXx8uvs8MStoC3ReMJz","pwf":{"c1":"hsNBBYkZ8kI4-nAfnJ4rXBfiCMVtHsWZ2uSTS--PS9A","c2":"C22jUE3lAnS-9AToyQ3_J8Tw1xuRsag7Lb_O9wiClis","r1":"HxnBdt694eMJFBbhsl2R_tHp-eq7XU9ht5zTPfazs3k","r2":"UmnFPP3KjJVvS3ha13bDPpQDPAt4liEFz8WgAD8J208"}},"Parry Hotter":{"R":"A0XndUhhhFdOK4gvjp2AF99Ag8RC7zdaI7VLWOAkuhYI","Z":"A-PhCJr7vdf9HuH5ICHDGuKJLK9taMvGRTqjAXy27aBI","pwf":{"c1":"lmRSyfRIl9XFVlZxaL4gQIHDuYcE6nRAGufU0mIxdtE","c2":"t0AZpVuCyJXtI2UOHNB35GVXQkisVIDNdF6MDC9PMbg","r1":"1d8ZeQYRqXqcGjaxC-f39s8qeR8a8hb3tqUf6TGtsWM","r2":"lVrXpvJIXiCWzHtqtG0D5UUDzkXVmMm87dXs5BJoW3A"}}},"pwf":{"a":"Ah2OXeqJicoOCTr86hzpgzYJm-Cmi0fe7NCbzlIrX_cT","b":"AjlW0CaXkaurwZ8C7rhugbCiLR09cn1pLs5PsvQt81s9","r":"L4ULesfPM8mQapBdao6auAeMSFrd9ScHDEzU7klz858"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a3"),"ballot_id":3,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924775749"}},"votes":{"Parry Hotter":{"R":"A9eXw_WA0o9lT50NcM1-yE2l6nvkNxs8LLOT-sQHeKS2","Z":"ArdgS7mqYecB66SA9oCJYTN4GgD8byhDKnDWSXK7vjeV","pwf":{"c1":"3RiXsU8CnpQmKdbIW_rELg3BUZb0DQZwbZ08gs0y2WA","c2":"Z1UadEIepvBk6YnE3Vl6DgG3NlBRX9xpaJge6CJaHog","r1":"Cr2DYEWWyQLynUI3i2osqkb_dv7J3XjZHuEwGpmMvNI","r2":"xcRImXLKufscgSgUNvD0aacIDYM3H6VIhWPu_0jHy34"}},"Chris Riches":{"R":"AnSNe2D6r2mWHTdkSBX2G_hApuP-ucboz0jYotnbwZg7","Z":"AghzQGBoTBqV7QniTqTqL-wnx0Hk5pPiOMwl_C1XXFIZ","pwf":{"c1":"RMczH3UFyqzQARFaZVDo4DNBxNX6jrLsb41h_1EIZ1c","c2":"eiV4_Is7rPDX9LX8nXgqwIJpq9ex1G7tL3sidQEZtqU","r1":"qDRGT73815OX3Wy9eb61GEjIW4lPLMd3uxz8iwzbV3o","r2":"w3NSCTvS99NgRnn6mdbO2AyGPEDdVfbEe0nwVo9T-FU"}}},"pwf":{"a":"AuYBinJ7S6FJea7pGe8lG62er1WOhIp65XDaBswwF91B","b":"AyKzBNW4qIRRXu3GlTRIIo7y6lExN7yb9gX7ggFv7mWd","r":"vQzPbkfJGWfCLMmDy-zICnnIe9fjN7h77agTlpTvEMs"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a4"),"ballot_id":4,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924775789"}},"votes":{"Chris Riches":{"R":"A2oAxlyBi9u-VqwhttZTVbp9JmsaWMyJlYbe988011cz","Z":"A6jwWcB9ozyFy2S7jayKQ12KPuO8mDbhTK-ge8DMqpww","pwf":{"c1":"WAvZxy9yTILwbA2y9ViqbVt6GXKOepa5xU2ilr_gHwA","c2":"K_Vuk8Rxo1yv3cAEjtp_o8CWAKb6io7PpmmCymq7O7U","r1":"pQA1wN6yeC2PE3yJYunYYunEA9xSFQYCfmA3eRhryjM","r2":"136vXuTFWPQbBtA79ciWv4f4tY0JBjXjlbGrElb5hnI"}},"Parry Hotter":{"R":"AopwZmGPqKcl5yIHAZmlUjBTjyU46v5D1R7tbBmOn5GH","Z":"ApwxGRe97GPtxmjM5TzqNXLg83NpKn95ipMfbF4fTprD","pwf":{"c1":"hCXm-pebGE5ACqTzSbzRf7fd91C-INAPhMjBfsJf8M8","c2":"d0KDuy8SJp4zt-U342fBzn2Pn62QgZr3Ei78xzvRXoc","r1":"E2Gae4cuXsTmYXgUzXYez7pAZ1wuHII6d4Lf7C8L9QE","r2":"_3QIQntKE-3Lsrt8Xld7y8zcWYEaH8weQkDs7We35as"}}},"pwf":{"a":"Al7UmfnB89RN3uxerBcpnn-71Q31kPo8R1RAOfpHVzWM","b":"AqD2WgIsWZCTrXxqxkKDCHVwZRAc-59VbG2Z3bw3vGp8","r":"zeRDLvGghLUhXibZZeLOktEhWsjGm3ZhDmoq8JSjki0"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a5"),"ballot_id":5,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924775828"}},"votes":{"Chris Riches":{"R":"A4MzpH_tYMwF2nFf4xxGK5yTY0jOvv9rxDWdGkTqCd_r","Z":"A2azK9Gu3sNA73xhjwY072payYRKdnSWu08DRi_wf-Z1","pwf":{"c1":"LdBUeFa7B6RTGUMo9xJSEh9Or1uOYLkn22MXelNQ68w","c2":"mfr_Q9YoXqaN2qvHxk2cdqVwQVJ7l241pd1TCoS3-Ak","r1":"PFAfsomk-iXKv1f-fzlE6Eh_yWypHNX_FMgwOrqlCHQ","r2":"OP63_gW644LEyP0A7_uYop0W4nOy4WlZePt34U5tSM4"}},"Parry Hotter":{"R":"A_87C3fNrGfDzHIbg5u10Nss-B1bbGiLn_u0r7BunP4r","Z":"Ar2jWX5-A2JjLkcbS_H9NEDNlZOEFlxifdZDjyt_Zi86","pwf":{"c1":"sE2oAL0CWspSx3koo_YS9h5Qt_at-8xoEZVAdN39syk","c2":"jhcU53EipPL91QbCztVLdHhoyLYggPCttPRRdsmGwJg","r1":"IuaufSR-lF9j0VdvXLjj8798I-Slc2-qtnAfM3ixPbU","r2":"QeLIEzsDKW6IKQu77Qr5cnE9q3QgxiRpObtvAQzrqzc"}}},"pwf":{"a":"AwNG2fjRMWQe8lAHMtehtEkOuaEJjkDZVXyI4epRjjfJ","b":"AlJ9u1uVZotlzOJVLl_6Q7OalPQXDMtzfiVCy5SJOaqh","r":"6N88sKs8yrFShUpdwZ_H6x8WB0PVfXdKMsJ1BVKl1cE"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a9"),"ballot_id":6,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924775987"}},"votes":{"Chris Riches":{"r":"7x5jkocue9ffMk7sC0Y03y4WzNLzSpTnKbgTDYu4tZA","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"AgyEgBxFt6oOY_j3212qZ5I5HAlgly8wt5_AvXJoq5fS","Z":"A6HtMPxvReCR82HYkNvKr8dgkpzJkc1SLiKaK4iY5IQp","pwf":{"c1":"1SNaPWEakliZVbSxWn8n-m8SDWGiOggbOCUkgNU5nPo","c2":"0mwaGOEsmUJpTE-wLQHQ-ibYljTLiGn93WrF83lVwWA","r1":"cVCZ2SAkffxvICIq2qyRCGV7a5uBe-xeyv69I-Lmg94","r2":"NL5RBGYmzIUM5EbXb9eKPyVGdwJI-x7SokTcC3uFduQ"}},"Parry Hotter":{"r":"2EDWvBtCJYHQL8qpJhUg57mPsh1X-qjwovwwQzIPdSc","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"AhSJ2k5WrxixZWaHkKeypFZJT6KB4SyDYRwswqtuVrtV","Z":"A2ckF799DVnbrABSDIlyIzX3pb1dmFEmHeI70tYB_Qgi","pwf":{"c1":"nAo2iaPmFIO1Gvg_cUo7Labbc3P-c_-1AaKEECrl2-E","c2":"AUk0Azd-Arx8UH3gcc0Wxs3v0cQ0wDsHltMU-8WHskQ","r1":"loVrJgm4SJGjtIkqd1mQEE6wOGcaBp5ICQm7DBqHoQo","r2":"Tx2S0Za9WlfsL5DngH3HQDgvRxCtYQ-9yvt6lYJoepc"}}},"pwf":{"a":"A-DRfGinapDGqFnS1zLmGWfM89l1lZ8GyxyMgtkuhMQk","b":"AhGakntD9g9PR5lewmR70w_UQB5Yb_2eMgLtcFVT3DTJ","r":"p_jkYjM2jGxLmh6sNc8D-fC37NVSWH7BwcYRHy7DagY"},"state":"Audited"},
  {"_id":ObjectId("62671ce89f44880257dd98aa"),"ballot_id":7,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924776027"}},"votes":{"Chris Riches":{"r":"vaZaxSvD70sogzKHYExvfSB9xmPQhQY7zGV7xOJQBPc","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"Akso-8_MR3P8CeTKtavZbGWp4DWd4px5SCfpfSInDTZp","Z":"AshG4npscW6jNWNXCy7tr-BQORMMNxj6yNbB7wqdiLbx","pwf":{"c1":"VyZUxky4uhqy2jcFtAamqxOjmv-bDjAwxI8gAAZDapo","c2":"dP_SBqcQf3s6PkPqrB5X4n1gi_SYlPMWGQz3LRIU9Tc","r1":"6snyYSlI-NdRdeqmnE0P_RlbJCZBfnajRr2eIuAJWT0","r2":"48O7cMgzWF882Y9KBwv8W_6Ehh-MbotFno6lPnQuLx0"}},"Parry Hotter":{"r":"MhQR8ap5KoehYzz2BbC6K_YTq9WrVJwzq74pNi7LTx0","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"Aup7TcrRKuC0ATN4XS-2dJ8Kk5B0phxw6uje6USPXhNQ","Z":"A6UZM6vMaLTKf1JqIB2A7DJJ3yzdJsjN8aVDL_BIg5sr","pwf":{"c1":"4dYhSG4hVhZQX1dA7Swl2K3s9s7o8b3Bc5iaod_P1uA","c2":"9u1vNt3a0IfdKqpMLHNo2Ll9P9fHTcwtz6cuJQ7tyGs","r1":"OSgxeyHotxe38b8UWbwWvBx3DpQon0SDzW1u8eVPwPI","r2":"BXmCQyfcoGZQ8jSmNoFRUKC3lYi3x7i-y6eSMM1es2k"}}},"pwf":{"a":"A3peaGCmZ_EiUpqoOFUuoIPEUrehPWycF2vn-Wqb7L6z","b":"A4B08BfpsVVugozyG-95kNZzhj-sLX3Ofum-57-HOqVQ","r":"Httn-WKFhLFNRoo34agBVzzSxtF5stR18BZFkNdKit8"},"state":"Audited"},
  {"_id":ObjectId("62671ce89f44880257dd98af"),"ballot_id":8,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924776225"}},"votes":{"Chris Riches":{"r":"CtRxQY-LmYiLppWoNnYD3gs-V1tcx2D3ZVrfUyyFkw8","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"A24KJPVb_V2sZ9k-3OvbZByrAqJJVRk49bNoE_yEG1YW","Z":"AnO1UwkwnMFbZnSpEtpoBYTWgSx1-DD2ob74L6jU1-X6","pwf":{"c1":"1JXAplZGG6nslhnceIrMPp-kreWGQj8ST6UMiRZhXTA","c2":"D4A3vicpHsWJ047ry-uFIe3MfvuB30mac--mozA4sfU","r1":"Rmes1FQ6m6eGUwsQvmLNOLFMMEYUQWDh7O3qg9O2gMM","r2":"VJ2Qyjvkdk_X9yBNyRRNNQPl7Dbh2j-soQTmqYme3GA"}},"Parry Hotter":{"r":"GEf3EGlIyrXwuv1wNtjj1GKALBCt6rQkvsTScZ1zaHw","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"A22nOuJR2nI0mOg8hnMeL_hkRQMmi8o9LmHkg2ojYeBW","Z":"A7tLFtDYUfx00lrf_4cCqWAjwQUMXGs9p0yMbz30eHpA","pwf":{"c1":"PComl9BI6VZ2ui6WEGeVHrJURHsBPGOS25pjMBq-nOA","c2":"S74SvbNfz_bBKKpw9Z1lQmNgEorv2wMah1rsgJoEsNg","r1":"UJrW_rtrLsECQcDbw_aEXNanlfNHatqbVcnss7dvxwg","r2":"ekaoUUt5olbqDlIJxSbYSqrV1Uewa-GbGaPt4nKGVUw"}}},"pwf":{"a":"A0iCcwC8gGNWUizM2U4CvaimtiwxI9abcMuSckin-sxX","b":"ArGnCeyhLmydlC-3Vbn1f_OqC5Q2Dol9lGzmyaXpPBAG","r":"nyaab-sHSo7H0fHmFxRcSqkIdZM56hoIXhh-Ngm8iZQ"},"state":"Unconfirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98b0"),"ballot_id":9,"election_id":1,"question_id":1,"creation_time":{"$date":{"$numberLong":"1650924776265"}},"votes":{"Chris Riches":{"r":"QIMziZKe57wf9qzuOof7-qgOSRUtwifOuasjdPyvUWY","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"A8nwzOQ5gNoU-LNiBIWga3lpqFuLemUyHw3JSNa7kpHo","Z":"AxLIVse7vXY7g85H_7GgcRI0NU5Czses2B2ae6c9sEcD","pwf":{"c1":"82VuVAi9L9m3EhB5oenpapa_vrr0SXg5ltrfI0_Ckd0","c2":"AQn-188VeaR070epef1RfUQCzl0Wp7CwalBPeIcInOI","r1":"BeAjMUQ3zxpozHTr-TgeWnm_gjSY3v8jykDhhrkHSjc","r2":"v-73DTud5g51-ec2uwb-GC-oK8e-ZHWgE0LsJqWYWcI"}},"Parry Hotter":{"r":"eyQKnjf36X9DlW5nU1BioaHNdb3P7n3OMz6KbkY7GIU","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"A9k2qUa-q0Q1WpdlZwNfu7-D37XyuZ2ZzENlvzl4B_eL","Z":"A8AHZPOxps51zHBxT7wbFq6kOwBpAA_nKx3SQySRPDBI","pwf":{"c1":"1yDXXkmvkC9MpIqJumQKEHu53hDH-fNZDHm5XMAFI7Q","c2":"bCSuApHq76uysHNvbf_s-HLb_3lQFVT-yRodoymwboY","r1":"0iKpve619dmD79EZnfjgjw01p4jE-k3RAlC9ZmWoLwc","r2":"2RsH7pVQNuacJAyKbRC0c3HsCLYtOP11sYYkpkUclC0"}}},"pwf":{"a":"Ap2fi1ZQFMxf34rLfplZwsiLtbMjnOuOFczf_xCnDFVW","b":"A7a6MkZwwrL1OiHO6qx54keeng_suB5fgs7OqeVQnCvX","r":"DVGuxIIdR0xobVpl3p1EgXoQWf_gTv09uNfXVewOGw4"},"state":"Unconfirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a6"),"ballot_id":1,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924775868"}},"votes":{"Jane Doe":{"R":"Ao9b-7IRTD7GNdvrh8HnsDWEcdj9Ow4Qr8pKIWhkctk-","Z":"AtsdPgEJvPLIyh66qexcjXe5e_OdumhyfEUjRuPEY-a5","pwf":{"c1":"gyOFehMkbKPFwh-CBSloK_BOKiKZOG9ng7nfGNFx6oY","c2":"pR5pX-QQR1U4A550O_9f4xwmPf8OSbBqgv87Mkw3MNc","r1":"3I5Dzol_18eHoUMFNvF-Ic3F95jAx8AyeyETExec1Qk","r2":"H0AL44lWtfnf_t_mtUwAVFsFpZwGBbk8F0YFsAj8-mo"}},"John Smith":{"R":"A-gvdKcWxYMdq6VB68yMGyainJDP5pCyPi-Twxg9uJui","Z":"At-7IF3ZYimiiSJ2vD8y6Vof8OcP8Sbuc5zRUMuBuv4c","pwf":{"c1":"szmr74dywjhQ5KwmJ5hwtAeNxI_c4LaoiybBh4Tj7Tk","c2":"7gWCJRDG2mKtLSKT7r5ByPpwLRj_67BNSRcrv71QWus","r1":"FrBXWZuWn1sBIP6k5qBfBG83Dqp85oSkie6BaaEGCgY","r2":"LZ6MHOcXkJ34CylCnKhg_yImrVp2GN6ImhxLTPp0o_U"}}},"pwf":{"a":"A_WZXWXd6keSWJRoUP0JFlMNyCUnS5YxO9b6FZ_ptCq6","b":"Aou2VyNYgqsAsrx4dVexcwQylVVqdF7ZR3SKIQvcsepY","r":"B0x0xeQwPzXBTtOXpqPBdw1M79d8KY3H1AlxdqJB2rY"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a7"),"ballot_id":2,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924775908"}},"votes":{"John Smith":{"R":"A9IomCd6FzmfXRfBi5JQn3B1mFOulziNdE8IPaHjuvFy","Z":"AjqUiaGfAbwHSIN2o_1DKvqKdk4dV6UGmtq70ULm6hIN","pwf":{"c1":"tkdrQhe8wvkFJqO59y1WC6KGcWd7szc1_vncPMqgKLE","c2":"O2jSXP4kPF2GNwzFCP6IJH1TCv2BEwGwEtOdKaO9XWA","r1":"bdedNFQOGnfFog60BxDfRDtX-Vl9BhFd_mtEHRUdqM8","r2":"-xKOoTe5tXrxUPl7bSZEAAxHmloVxAcziGa3QMmGsiY"}},"Jane Doe":{"R":"AmqjfZR5i8_NOd0p1kiqqDDj8dcWmr-LO8SLNYZ5NiZc","Z":"Ar3eDGJq3m90tXdrjZlAUQV2BsV7t6elHk90qtDg_1mz","pwf":{"c1":"Qneor0djuba0I7AqtYL67L1fke8EGW4q3VCqLcnVEWU","c2":"Ke9QA367cItu7VfByqvN-WmoGR1nPjWXTQZMIcumT_M","r1":"ZJlg4MOnXIempsPCTQ3jB_LHMxDaxVGLOLYkwGylhgM","r2":"WQ5qt8zL9l01FRquxHIgeWH6o76JRqz2XrMd8zG4YtY"}}},"pwf":{"a":"A2TuUqI508JefOXTvBznSJK3bY77gIRwSxRO9vq0JRaB","b":"AwBY-uqaqnEjMNF_KQASOLgYfltsAJdETCxl1U2ObvCR","r":"EktTgkKEZm5_DyGqUyKJQA2YeVIpMuW74vwmS6cd9mc"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98a8"),"ballot_id":3,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924775947"}},"votes":{"John Smith":{"R":"Ak6JoQ2irrFNGJjwp0HONJ2UuElt4QB8cPD4GguWkxk7","Z":"AoqT3EXKA3vhx0Uh7aS17-N3srsh7rBuTWLzgm9G7_eC","pwf":{"c1":"LpiE2pvMMEE1YClUjxuBV6s_g_GpRWoG0fKFxtzrlJg","c2":"hk95fAGSiQn9KyNUC8-nq_92q2M4r7XVCsqEuU2ewYg","r1":"aBUvHQcRp867h1u3tJlMApIKkxnQP7pJYsf5ziwEFgU","r2":"BM71NMxTMXov2s7kzTQ07eWffEpFGDV8tUh6iO6jXE8"}},"Jane Doe":{"R":"A4UgMDAwZDfLs7YwjU9paQoKA6I5xBlIZEgWBgQHo-yS","Z":"A9i5wQJc_XzX9AUDig8Izl3c7NxdERsHtCGpQImpgrN1","pwf":{"c1":"1e4hB5RYgANW_rDxK5Vzj718Ggns-gAFykfXPBSRLY4","c2":"DPXCTe4eLNL8MFgrMs9Y83nShn3JsoRHrGaNdkRbt-o","r1":"as214IJ8YZUhBhTrflemitsaVrzXyyQkkZhQO54FuD4","r2":"fW1Fpp-3U1pCUp6jMCzL9OUMnYK608pUeQSo-8NlEm8"}}},"pwf":{"a":"AyKSE_ZHT-TEnghNiZUuvY-LvF8eCDiBCMfNB-c_-ojX","b":"Ah7EKYKoWZDjzr5wVZZjzKfvEXw-5zsCNCetHsle5XTg","r":"VMBjBD8QUx_KxjVxf_412kewWxpvX0eq_oWaG3hbdnk"},"state":"Confirmed"},
  {"_id":ObjectId("62671ce89f44880257dd98ab"),"ballot_id":4,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924776067"}},"votes":{"Jane Doe":{"r":"ZKf98GRoSZIu8lVpG-c3CJbKldmGSKIW3CFOgY_cYlg","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"AmTdmCjwk5WH5pKXQVGQdkXLjQlAErFkxJvOOxUgahQE","Z":"AoFPoZi1pQWCwBHjNn5gJNo-dkwo1c6r5tbILA0Yu40v","pwf":{"c1":"wjeXbeI3OvG5ytZg8y3Fim9IGV7tvoy008Zz7bUXNkI","c2":"_F02u7IZ1aWTiTSjrfvvxXcOq552fezAUOEtpmjsJsQ","r1":"ZnUtJXYBOhhquXmSPdkYbGa7AXyyWTH9VGOGa6iDI9Y","r2":"GJboCIDzApBkZB55oTb_XKzmoLvnG2xx-UcVLU29g_k"}},"John Smith":{"r":"K8kMs6h8A1w9YqH6tjNrJE3abVlb92Cbnkj4MCUyQFs","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"AzbV6_ElbjqPYgHiijy9Vl_m6l8eXsEna9Zmw9QPPqVq","Z":"A3slRu3hFpcP5xDgbTPERZ2VFfKFPAr5-gwsdIYJBvx4","pwf":{"c1":"Y49UiuZrFKOo5fcocPgWOXo0_y2MLPWwHwz73FPuUAY","c2":"lQixcMJJR6FAlz0MGGse32ZWZRswmVSkKJdSMNo-0Jc","r1":"aLMh9ResCkdw1G-AQn83VwhtiPxTqwWj5LniEfllTm8","r2":"KFFcb3nz-1X4FN0W2VFPsJaTBclB23P6SnwBSEq8MMI"}}},"pwf":{"a":"A4D49BlQLqw0WTSJCL7IWqW0V_8tTE_U5FeANUsVKA0q","b":"ArdSc-Dhj1b-7-o-CmnVqcdMkA645lOShcgt2M1p9Si6","r":"wUhfNl3ihjBGtTgq1f8Hf6YKQLP2PTp4kA4GSeyLKHI"},"state":"Audited"},
  {"_id":ObjectId("62671ce89f44880257dd98ac"),"ballot_id":5,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924776107"}},"votes":{"John Smith":{"r":"NM4wrstq2tlQujNvI1NU4iHIQcLGAuKqSPX9NMoH_rs","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"A5tGF2vFxUbxn9Rj68VOpXRvAeivEcNY3fpOADGuG6oh","Z":"An6-Gd_exChm0mf5CGcaoI-KvIsVfHq7rU3DSM694FOH","pwf":{"c1":"V6R2HrqYhmlVRKxcK5EDJR-TAj-pKdKTv5-1RwP-C3I","c2":"wGwc6MiPw20vdbuXBb3A6RilxBShQSVZhXNiLDLyVbM","r1":"K6qYQWpzCiFKycZp5whoqTL16jvyYN4OwhSrnv5UlxA","r2":"GZbpKPdcpP8rQwtMKBfBU8G3M0Gx_50ZM-etALl7Ou4"}},"Jane Doe":{"r":"BJzW0NByUC92qGwZUezTdYWaXAei8mAz2o5N_-K-zY0","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"A4m0GZjhm1evUMEVZ1H0Q4T_-6kCQRsPBBeA7t-MhSD5","Z":"AkhI_4YjOxWGwuq03IRk0cg6lCDR_V1l7g0ISYU0uQFy","pwf":{"c1":"v74wCvUf0nwcasY14f9bjaUQ6x4wRNDfYWkxPJhmteQ","c2":"MVlUclhcDebKfQilwYW_fVlv0n3rkliQdxtI6TelkBk","r1":"UxwSzAahdV-GHABcnf_b16OnlyUPPPaWOF128QdOkwE","r2":"kK-3cKEI0EQp9hHT42UdBxjflWIe6zs7QtIj-oCQifg"}}},"pwf":{"a":"A_WD8lzdhEeTuvOT5fvG1mWdoPtzD0bGZTWBQ5Q91DqB","b":"AsOWsBSuggPZnUZbom1t5DveYn6NPM563gfcAFQxirUN","r":"aD3boFltfSRUa-ylzDtD9XJ0BXgCnDASlSyIOGn7HfQ"},"state":"Audited"},
  {"_id":ObjectId("62671ce89f44880257dd98ad"),"ballot_id":6,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924776146"}},"votes":{"Jane Doe":{"r":"p2MRkZ9GiGrH0avPp7bOJ9dHELl36eXrUwJ24KUb3Z0","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"AwQHfe7CR1KQZJEa4Q2JNARhx36yNweRVdNJhDl5Wu6O","Z":"AnQsIYvh3Z4UNejFZiFS63OlKrBLNYP1VqhjeeYQq2lw","pwf":{"c1":"0OzV7zWqBesTZtEnNdZWM5PkovrTVn3nhyz4e82bMRc","c2":"PhEi2U7NKyRwJZjLsnsBnJ7sIfJIuuP1McPCzA7bHzY","r1":"Ok24gPRrCjqECZUqMGMe-rctd3Pi71a6lteUMmVDCrs","r2":"hQVMd2-vB9V1wJPLouURfXIgZ_mxF4domsW4G8hRoQw"}},"John Smith":{"r":"WbH26RjV00U2-6vU0Kyt0IBkUCuW_n2g1BANYEVMI2I","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"AqIRC4X40BM8xVyXOg4LEcjDaLSr_Gx-t_0mB6NeAK9L","Z":"AnsbK2z8ZnrAoQAROoXASnAgvk4TiAHHoECpMAPhNVjN","pwf":{"c1":"f4-LMOH4U81OOvMP3GigNZg4VmvnGr5Arw-7l7pYnXM","c2":"cCEzPZ1TJa7SKkQLaXvXo7D71w6mGAyyrzBTZmlU9dE","r1":"A6_nFbSrQ_73R1G2dHki_Z3dDst3uVyw90sWPFQ7KBc","r2":"E9TBGILsrS2TkItnODNERIMOqoJNsccWoX5zly78OYQ"}}},"pwf":{"a":"Ap6V8QOWYDp4elR0RQGoQl24yjTAP6VOjL6lAVZ6EvyU","b":"A9AZfFiBQk5OvMHo69__ReNpF1ctxdfXh4pFSeb5cdtJ","r":"SyDcOZPNBtW5EgYNoc_Rna7y39TNLsrfp1t00LlEKTI"},"state":"Audited"},
  {"_id":ObjectId("62671ce89f44880257dd98ae"),"ballot_id":7,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924776186"}},"votes":{"Jane Doe":{"r":"4lL0IH2oB7r8d_jLbXLWuJ3yhu8yv9bem0t-X991ea8","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"AkscdzBGUsrTlAVzR3Kyly01szJbAfpR4sl4lgQG7v1C","Z":"A1iWqhPQPR_UWWz_d8-Ek3NrjnCYK2VeqfpxkeVsIJM0","pwf":{"c1":"Udg4tzQlEdVXfqc3N4c_0mLukgIpJGIdUGCTLHufp-w","c2":"_lbfsZpRSbR4_A-xMPtCCHjK6WPonpsx7aJ4-qr_9jw","r1":"W6vm9lIFF5q9GVe1J-pFDTdM1YdXbWkf1sScpv0du2U","r2":"fvWRLWOd7IuLNrxEG1emhmLJEjAY3B1g505WrzG_trI"}},"John Smith":{"r":"vfyjv5X0KyZu1GuMYaS948vm_gTE5W_vXn9Mp7M0cC4","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"A47KefcmrzYFA_KfDGiGTXpuKgWkq9o4aMqS_enolkyD","Z":"AnSrEQoDoM_MNJqdEx33o_MUd8G2HvTKd2luKPxwGhhz","pwf":{"c1":"9Mj7vnffn3Z0mhu5t5LVTpyX7qotOMaFzHUcobE7DGI","c2":"OaEy9gIYCgqXqyjYb24TfRnSIzGDxikWy9rZpMkGwF0","r1":"iuOhH2E04TuWis-P7rD1bY1DHgsxzV5ldgS-Zim8wAw","r2":"im7Dy-rJTDYunk65NNDCCqDLSi0GvPrWJabdc131MY8"}}},"pwf":{"a":"A-whjo58mVQ2V_bJqVSdoGEDKfyZDgeJI-0GjOUnDzAH","b":"AwHe6JxyzzohZIpq0aS9DWHiXnBUkTXTd9gT6i7HlCb9","r":"rwoc_zAaXpstaaOG9BQ7YHXrtqN_jg7n1oXlItcaru8"},"state":"Audited"},
  {"_id":ObjectId("62671ce89f44880257dd98b1"),"ballot_id":8,"election_id":1,"question_id":2,"creation_time":{"$date":{"$numberLong":"1650924776305"}},"votes":{"Jane Doe":{"r":"ax_dpYhM4PHoKAtwlR-8TCSbz9RkdHf34oc26JD7CZk","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","R":"Ars2g996-9LbjSIFNr3L-6MEh5bHurlCFlLK_J_24zrY","Z":"AoX306JRq3jQr5OsSJfLC_pOg4ueYLWSlew3SxXX8ox-","pwf":{"c1":"JlzyZ98LJvHYaJ9kfjN78BRwjJQ5a1V7CxQzJWh6f1U","c2":"xKv7tu0gZYw7Eerlp8R3e72aYs9VTMSI4GdKqHrSACI","r1":"-CTZ-Xn7ROxch-97G1B1ESMktXRTwFvKhFK0yFQCOo4","r2":"j2zjXaxrfDQM4wvfOJYorLlCylCpW5NENs_AlcnXmPw"}},"John Smith":{"r":"Hx_eQr8Cpz0_1LENjQ5ugq3C8sG__57mggeE-NzcUj0","v":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE","R":"A_OKJivVR8hy4DYoyTXAMUcCwW5o46M6R1_CDRhIFAZX","Z":"AnXEpdRAtGGpOLUwhnx5ZKHNuxgGcMDLPkADhiuzNdoo","pwf":{"c1":"IxUSnw_FtycY91UBYmwb4u1co5Quv81neeQ7_3k8bYI","c2":"GLHr2C9ZcLZfMvyAEnUkvUE-MO2yMac3oF1_aGRPF6Y","r1":"R-SF9w6DMRKIpFi9e5TejzSKSBaRDIoYBNilaiIvqYo","r2":"1czEdN6JnZseNjalbOsnYXnNTF8JBgXc5S2aY6M1C6Q"}}},"pwf":{"a":"Ap1iVVUYgZ3GOxgaMdWxUgEzfuw7yWqAICV3xCB-dGCI","b":"A91jtrV_SUWPLqAKMfMkXpN1zivCgBX7N1cZ5UGHeCv6","r":"c0npGJEpoX5l2aKe_3qA2ET3JbdMIEfVtKoN6K5s0oo"},"state":"Unconfirmed"}
]);

// Add the corresponding candidate totals.
db.candidate_totals.insertMany([
  {"_id":ObjectId("62671ce89f44880257dd98b4"),"election_id":1,"question_id":2,"candidate_name":"John Smith","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","r_sum":"EMwWOPaL1N3GsrC5NtYUMOCDhAoRD8NYhJ-AsGSrX6U"},
  {"_id":ObjectId("62671ce89f44880257dd98b5"),"election_id":1,"question_id":2,"candidate_name":"Jane Doe","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM","r_sum":"tgTpzAImpxwMrc1g5NfQsqpN6Z5neQPUtYZzdvGKeH0"},
  {"_id":ObjectId("62671ce89f44880257dd98b2"),"election_id":1,"question_id":1,"candidate_name":"Chris Riches","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM","r_sum":"rx8EnfogmHQ-ij7YoGE_fncu42-hOK1CaeL_f7-ecSY"},
  {"_id":ObjectId("62671ce89f44880257dd98b3"),"election_id":1,"question_id":1,"candidate_name":"Parry Hotter","tally":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAI","r_sum":"YIg-nQLD-WyGXB8T2LBEJi7p1-mSdnHzAJ7YnVUeZZw"}
]);

// Create the counters.
db.counters.insertMany([
  {
    "_id": "eid",
    "next": 3
  },
  {
    "_id": "bid:1:1",
    "next": 10
  },
  {
    "_id": "bid:1:2",
    "next": 9
  },
  {
    "_id": "bid:2:1",
    "next": 1
  },
  {
    "_id": "bid:2:2",
    "next": 1
  },
  {
    "_id": "bid:2:3",
    "next": 1
  }
]);
